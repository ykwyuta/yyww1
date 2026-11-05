//! Direct3D11（地形・ユニット描画）と Direct2D/DirectWrite（文字・UI）による描画層

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::Fxc::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// ゲーム側は描画層の画像を `gfx::Bitmap` として扱う（ブラウザ版と共通）
pub type Bitmap = ID2D1Bitmap;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    pub c: [f32; 4],
}

const SHADER: &str = r#"
cbuffer CB : register(b0) { float4 xform; };
struct VSIn { float2 pos : POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
struct PSIn { float4 pos : SV_POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
PSIn vs_main(VSIn i) {
    PSIn o;
    o.pos = float4(i.pos * xform.xy + xform.zw, 0.0, 1.0);
    o.uv = i.uv;
    o.col = i.col;
    return o;
}
Texture2D tex : register(t0);
SamplerState smp : register(s0);
float4 ps_color(PSIn i) : SV_TARGET { return i.col; }
float4 ps_tex(PSIn i) : SV_TARGET { return tex.Sample(smp, i.uv) * i.col; }
"#;

const MAX_VERTS: usize = 240_000;

pub struct Texture {
    pub tex: ID3D11Texture2D,
    pub srv: ID3D11ShaderResourceView,
    pub w: u32,
    pub h: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Font {
    Small,
    Normal,
    NormalCenter,
    Big,
    BigCenter,
    Title,
    /// 題字（Title が扁額に収まらないとき）
    TitleMid,
}

pub struct Gfx {
    pub device: ID3D11Device,
    pub ctx: ID3D11DeviceContext,
    swapchain: IDXGISwapChain,
    rtv: Option<ID3D11RenderTargetView>,
    vs: ID3D11VertexShader,
    ps_color: ID3D11PixelShader,
    ps_tex: ID3D11PixelShader,
    layout: ID3D11InputLayout,
    vb: ID3D11Buffer,
    cb: ID3D11Buffer,
    blend: ID3D11BlendState,
    samp_lin: ID3D11SamplerState,
    samp_pt: ID3D11SamplerState,
    raster: ID3D11RasterizerState,
    hwnd: HWND,
    /// 実ピクセル寸法
    pw: u32,
    ph: u32,
    /// UI 座標系（実ピクセル / ui）での寸法。ゲーム側の画面座標はすべてこちら
    pub width: u32,
    pub height: u32,
    /// UI 拡大率（画面の高さに比例 × 利用者設定）
    pub ui: f32,
    pub user_scale: f32,
    /// 全画面化する前のウィンドウスタイルと位置
    saved_window: Option<(isize, RECT)>,
    pub verts: Vec<Vertex>,
    xform: [f32; 4],
    d2d_factory: ID2D1Factory,
    d2d: Option<ID2D1RenderTarget>,
    brush: Option<ID2D1SolidColorBrush>,
    dwrite: IDWriteFactory,
    fonts: Vec<IDWriteTextFormat>,
    /// 文字の計測結果（文・書体・幅 → 寸法）。書体を作り直したら捨てる
    measure_cache: std::collections::HashMap<(String, u8, u32), f32>,
    /// 文字の配置（文・書体・枠の幅と高さ → 配置）。書体を作り直したら捨てる
    layout_cache: std::collections::HashMap<(String, u8, u32, u32), IDWriteTextLayout>,
    pub d2d_gen: u32,
    wbuf: Vec<u16>,
    /// スマートフォン向けの詰めた画面（Windows 版では常に false。ブラウザ版と共通のゲーム側のため）
    pub compact: bool,
}

fn blob_bytes(b: &ID3DBlob) -> &[u8] {
    unsafe { std::slice::from_raw_parts(b.GetBufferPointer() as *const u8, b.GetBufferSize()) }
}

/// 使っているグラフィックスの情報をログに書く
fn log_adapter(device: &ID3D11Device) {
    let desc = device.cast::<IDXGIDevice>().and_then(|d| unsafe { d.GetAdapter()?.GetDesc() });
    if let Ok(d) = desc {
        let n = d.Description.iter().position(|&c| c == 0).unwrap_or(d.Description.len());
        crate::crashlog::line(&format!("[INFO] GPU {} VRAM {}MB 機能レベル {:?}", String::from_utf16_lossy(&d.Description[..n]), d.DedicatedVideoMemory / (1024 * 1024), unsafe { device.GetFeatureLevel() }));
    }
}

fn compile(src: &str, entry: PCSTR, target: PCSTR) -> Result<ID3DBlob> {
    let mut code = None;
    let mut err = None;
    let r = unsafe { D3DCompile(src.as_ptr() as _, src.len(), PCSTR::null(), None, None, entry, target, D3DCOMPILE_OPTIMIZATION_LEVEL3, 0, &mut code, Some(&mut err)) };
    if let Err(e) = r {
        if let Some(err) = err {
            let msg = String::from_utf8_lossy(blob_bytes(&err)).to_string();
            panic!("shader compile error: {msg}");
        }
        return Err(e);
    }
    Ok(code.unwrap())
}

/// いまの言語の書体で文字の書式を作る（日本語は游ゴシック、中国語は微軟雅黒、英語は Segoe UI）
fn make_fonts(dwrite: &IDWriteFactory) -> Result<Vec<IDWriteTextFormat>> {
    let (family, locale) = match crate::lang::lang() {
        crate::lang::Lang::Ja => (w!("Yu Gothic UI"), w!("ja-jp")),
        crate::lang::Lang::Zh => (w!("Microsoft YaHei UI"), w!("zh-cn")),
        crate::lang::Lang::En => (w!("Segoe UI"), w!("en-us")),
    };
    let specs: [(f32, bool, bool); 7] = [
        (12.0, false, false),
        (14.0, false, false),
        (14.0, true, false),
        (18.0, false, true),
        (20.0, true, true),
        (40.0, true, true),
        // TitleMid：題字が扁額に収まらないとき
        (30.0, true, true),
    ];
    let mut fonts = Vec::new();
    for (size, center, bold) in specs {
        unsafe {
            let f = dwrite.CreateTextFormat(
                family,
                None,
                if bold { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL },
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                locale,
            )?;
            if center {
                f.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
                f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            }
            f.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
            fonts.push(f);
        }
    }
    Ok(fonts)
}

impl Gfx {
    pub fn new(hwnd: HWND, width: u32, height: u32) -> Result<Gfx> {
        unsafe {
            let desc = DXGI_SWAP_CHAIN_DESC {
                BufferDesc: DXGI_MODE_DESC { Width: width, Height: height, Format: DXGI_FORMAT_B8G8R8A8_UNORM, ..Default::default() },
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                OutputWindow: hwnd,
                Windowed: TRUE,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                Flags: 0,
            };
            let mut swapchain = None;
            let mut device = None;
            let mut ctx = None;
            let levels = [D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_10_0];
            let mut r = D3D11CreateDeviceAndSwapChain(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&levels),
                D3D11_SDK_VERSION,
                Some(&desc),
                Some(&mut swapchain),
                Some(&mut device),
                None,
                Some(&mut ctx),
            );
            if r.is_err() {
                r = D3D11CreateDeviceAndSwapChain(
                    None,
                    D3D_DRIVER_TYPE_WARP,
                    HMODULE::default(),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    Some(&levels),
                    D3D11_SDK_VERSION,
                    Some(&desc),
                    Some(&mut swapchain),
                    Some(&mut device),
                    None,
                    Some(&mut ctx),
                );
            }
            r?;
            let device: ID3D11Device = device.unwrap();
            let ctx: ID3D11DeviceContext = ctx.unwrap();
            let swapchain: IDXGISwapChain = swapchain.unwrap();
            log_adapter(&device);

            let vsb = compile(SHADER, s!("vs_main"), s!("vs_4_0"))?;
            let psc = compile(SHADER, s!("ps_color"), s!("ps_4_0"))?;
            let pst = compile(SHADER, s!("ps_tex"), s!("ps_4_0"))?;
            let mut vs = None;
            device.CreateVertexShader(blob_bytes(&vsb), None, Some(&mut vs))?;
            let mut ps_color = None;
            device.CreatePixelShader(blob_bytes(&psc), None, Some(&mut ps_color))?;
            let mut ps_tex = None;
            device.CreatePixelShader(blob_bytes(&pst), None, Some(&mut ps_tex))?;
            let elems = [
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("POSITION"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 0,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("TEXCOORD"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 8,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("COLOR"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: 16,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
            ];
            let mut layout = None;
            device.CreateInputLayout(&elems, blob_bytes(&vsb), Some(&mut layout))?;

            let mut vb = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: (MAX_VERTS * std::mem::size_of::<Vertex>()) as u32,
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
                    MiscFlags: 0,
                    StructureByteStride: 0,
                },
                None,
                Some(&mut vb),
            )?;
            let mut cb = None;
            device.CreateBuffer(
                &D3D11_BUFFER_DESC {
                    ByteWidth: 16,
                    Usage: D3D11_USAGE_DYNAMIC,
                    BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
                    CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
                    MiscFlags: 0,
                    StructureByteStride: 0,
                },
                None,
                Some(&mut cb),
            )?;
            let mut bd = D3D11_BLEND_DESC::default();
            bd.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
                BlendEnable: TRUE,
                SrcBlend: D3D11_BLEND_SRC_ALPHA,
                DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOp: D3D11_BLEND_OP_ADD,
                SrcBlendAlpha: D3D11_BLEND_ONE,
                DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                BlendOpAlpha: D3D11_BLEND_OP_ADD,
                RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
            };
            let mut blend = None;
            device.CreateBlendState(&bd, Some(&mut blend))?;
            let mk_samp = |f: D3D11_FILTER| -> Result<ID3D11SamplerState> {
                let sd = D3D11_SAMPLER_DESC {
                    Filter: f,
                    AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                    AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                    AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                    MipLODBias: 0.0,
                    MaxAnisotropy: 1,
                    ComparisonFunc: D3D11_COMPARISON_NEVER,
                    BorderColor: [0.0; 4],
                    MinLOD: 0.0,
                    MaxLOD: f32::MAX,
                };
                let mut s = None;
                device.CreateSamplerState(&sd, Some(&mut s))?;
                Ok(s.unwrap())
            };
            let samp_lin = mk_samp(D3D11_FILTER_MIN_MAG_MIP_LINEAR)?;
            let samp_pt = mk_samp(D3D11_FILTER_MIN_MAG_MIP_POINT)?;
            let rd = D3D11_RASTERIZER_DESC { FillMode: D3D11_FILL_SOLID, CullMode: D3D11_CULL_NONE, DepthClipEnable: TRUE, ..Default::default() };
            let mut raster = None;
            device.CreateRasterizerState(&rd, Some(&mut raster))?;

            let d2d_factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let fonts = make_fonts(&dwrite)?;

            let mut g = Gfx {
                device,
                ctx,
                swapchain,
                rtv: None,
                vs: vs.unwrap(),
                ps_color: ps_color.unwrap(),
                ps_tex: ps_tex.unwrap(),
                layout: layout.unwrap(),
                vb: vb.unwrap(),
                cb: cb.unwrap(),
                blend: blend.unwrap(),
                samp_lin,
                samp_pt,
                raster: raster.unwrap(),
                hwnd,
                pw: width,
                ph: height,
                width,
                height,
                ui: 1.0,
                user_scale: 1.15,
                saved_window: None,
                verts: Vec::with_capacity(MAX_VERTS),
                xform: [1.0, 1.0, 0.0, 0.0],
                d2d_factory,
                d2d: None,
                brush: None,
                dwrite,
                fonts,
                measure_cache: std::collections::HashMap::new(),
                layout_cache: std::collections::HashMap::new(),
                d2d_gen: 0,
                wbuf: Vec::new(),
                compact: false,
            };
            g.create_targets()?;
            g.update_scale();
            Ok(g)
        }
    }

    fn create_targets(&mut self) -> Result<()> {
        unsafe {
            let bb: ID3D11Texture2D = self.swapchain.GetBuffer(0)?;
            let mut rtv = None;
            self.device.CreateRenderTargetView(&bb, None, Some(&mut rtv))?;
            self.rtv = rtv;
            let surf: IDXGISurface = self.swapchain.GetBuffer(0)?;
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                dpiX: 96.0,
                dpiY: 96.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let rt = self.d2d_factory.CreateDxgiSurfaceRenderTarget(&surf, &props)?;
            rt.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            let brush = rt.CreateSolidColorBrush(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None)?;
            self.d2d = Some(rt);
            self.brush = Some(brush);
            self.d2d_gen += 1;
        }
        Ok(())
    }

    /// UI 拡大率を決め、UI 座標系での画面寸法を更新する。
    /// 画面の高さ 900px を基準に拡大し、UI が収まる最小寸法（1360×720）を下回らないよう制限する。
    pub fn update_scale(&mut self) {
        let (pw, ph) = (self.pw as f32, self.ph as f32);
        let cap = (ph / 720.0).min(pw / 1360.0).max(0.5);
        self.ui = (ph / 900.0 * self.user_scale).clamp(0.5, cap);
        self.width = ((pw / self.ui).round() as u32).max(1);
        self.height = ((ph / self.ui).round() as u32).max(1);
    }

    /// スマートフォン向けの画面（Windows 版には無い）
    pub fn set_compact(&mut self, _c: bool) {}

    /// 文字・UI の描画を縦にずらす（スマートフォン向けの一覧のスクロール用。Windows 版では使わない）
    pub fn set_2d_offset(&mut self, _dy: f32) {}

    /// 利用者の文字サイズ設定を変える（d: +1 で拡大、-1 で縮小）
    pub fn adjust_scale(&mut self, d: i32) {
        self.user_scale = (self.user_scale * 1.12f32.powi(d)).clamp(0.6, 2.5);
        self.update_scale();
    }

    /// ボーダーレス全画面とウィンドウ表示を切り替える
    pub fn toggle_fullscreen(&mut self) {
        unsafe {
            let hwnd = self.hwnd;
            if let Some((style, rc)) = self.saved_window.take() {
                SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                let _ = SetWindowPos(hwnd, None, rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top, SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOOWNERZORDER);
            } else {
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let mut rc = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rc);
                let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
                if !GetMonitorInfoW(mon, &mut mi).as_bool() {
                    return;
                }
                self.saved_window = Some((style, rc));
                SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !(WS_OVERLAPPEDWINDOW.0 as isize)) | WS_POPUP.0 as isize | WS_VISIBLE.0 as isize);
                let m = mi.rcMonitor;
                let _ = SetWindowPos(hwnd, Some(HWND_TOP), m.left, m.top, m.right - m.left, m.bottom - m.top, SWP_FRAMECHANGED | SWP_NOOWNERZORDER);
            }
        }
    }

    pub fn is_fullscreen(&self) -> bool {
        self.saved_window.is_some()
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 || (w == self.pw && h == self.ph) {
            return;
        }
        self.rtv = None;
        self.d2d = None;
        self.brush = None;
        unsafe {
            self.ctx.OMSetRenderTargets(None, None);
            self.ctx.Flush();
            match self.swapchain.ResizeBuffers(0, w, h, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0)) {
                Ok(()) => {
                    self.pw = w;
                    self.ph = h;
                }
                Err(e) => self.warn_device("ResizeBuffers", &e),
            }
        }
        if let Err(e) = self.create_targets() {
            self.warn_device("create_targets", &e);
        }
        self.update_scale();
    }

    pub fn create_texture(&self, w: u32, h: u32, data: &[u32]) -> Result<Texture> {
        unsafe {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: w,
                Height: h,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };
            let init = D3D11_SUBRESOURCE_DATA { pSysMem: data.as_ptr() as _, SysMemPitch: w * 4, SysMemSlicePitch: 0 };
            let mut tex = None;
            self.device.CreateTexture2D(&desc, Some(&init), Some(&mut tex))?;
            let tex = tex.unwrap();
            let mut srv = None;
            self.device.CreateShaderResourceView(&tex, None, Some(&mut srv))?;
            Ok(Texture { tex, srv: srv.unwrap(), w, h })
        }
    }

    pub fn update_texture(&self, t: &Texture, x: u32, y: u32, w: u32, h: u32, data: &[u32]) {
        unsafe {
            let bx = D3D11_BOX { left: x, top: y, front: 0, right: x + w, bottom: y + h, back: 1 };
            self.ctx.UpdateSubresource(&t.tex, 0, Some(&bx), data.as_ptr() as _, w * 4, 0);
        }
    }

    pub fn begin(&mut self, clear: [f32; 4]) {
        unsafe {
            let Some(rtv) = &self.rtv else { return };
            self.ctx.OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
            self.ctx.ClearRenderTargetView(rtv, &clear);
            let vp = D3D11_VIEWPORT { TopLeftX: 0.0, TopLeftY: 0.0, Width: self.pw as f32, Height: self.ph as f32, MinDepth: 0.0, MaxDepth: 1.0 };
            self.ctx.RSSetViewports(Some(&[vp]));
            self.ctx.RSSetState(&self.raster);
            self.ctx.OMSetBlendState(&self.blend, None, 0xffffffff);
            self.ctx.IASetInputLayout(&self.layout);
            self.ctx.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            let stride = std::mem::size_of::<Vertex>() as u32;
            let offset = 0u32;
            self.ctx.IASetVertexBuffers(0, 1, Some(&Some(self.vb.clone())), Some(&stride), Some(&offset));
            self.ctx.VSSetShader(&self.vs, None);
            self.ctx.VSSetConstantBuffers(0, Some(&[Some(self.cb.clone())]));
        }
        self.verts.clear();
    }

    /// world -> NDC 変換: ndc = pos * scale + offset
    pub fn set_xform(&mut self, sx: f32, sy: f32, ox: f32, oy: f32) {
        self.flush();
        self.xform = [sx, sy, ox, oy];
        unsafe {
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            if self.ctx.Map(&self.cb, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut m)).is_ok() {
                std::ptr::copy_nonoverlapping(self.xform.as_ptr(), m.pData as *mut f32, 4);
                self.ctx.Unmap(&self.cb, 0);
            }
        }
    }

    pub fn set_screen_xform(&mut self) {
        let (w, h) = (self.width as f32, self.height as f32);
        self.set_xform(2.0 / w, -2.0 / h, -1.0, 1.0);
    }

    fn upload_draw(&mut self) {
        if self.verts.is_empty() {
            return;
        }
        unsafe {
            let mut start = 0;
            while start < self.verts.len() {
                let n = (self.verts.len() - start).min(MAX_VERTS / 3 * 3);
                let mut m = D3D11_MAPPED_SUBRESOURCE::default();
                if self.ctx.Map(&self.vb, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut m)).is_ok() {
                    std::ptr::copy_nonoverlapping(self.verts.as_ptr().add(start), m.pData as *mut Vertex, n);
                    self.ctx.Unmap(&self.vb, 0);
                    self.ctx.Draw(n as u32, 0);
                }
                start += n;
            }
        }
        self.verts.clear();
    }

    /// 単色三角形バッチを描画
    pub fn flush(&mut self) {
        if self.verts.is_empty() {
            return;
        }
        unsafe {
            self.ctx.PSSetShader(&self.ps_color, None);
        }
        self.upload_draw();
    }

    pub fn draw_textured(&mut self, t: &Texture, x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 4], linear: bool) {
        self.flush();
        unsafe {
            self.ctx.PSSetShader(&self.ps_tex, None);
            self.ctx.PSSetShaderResources(0, Some(&[Some(t.srv.clone())]));
            let s = if linear { self.samp_lin.clone() } else { self.samp_pt.clone() };
            self.ctx.PSSetSamplers(0, Some(&[Some(s)]));
        }
        let v = |x, y, u, vv| Vertex { x, y, u, v: vv, c };
        self.verts.extend_from_slice(&[v(x0, y0, 0.0, 0.0), v(x1, y0, 1.0, 0.0), v(x1, y1, 1.0, 1.0), v(x0, y0, 0.0, 0.0), v(x1, y1, 1.0, 1.0), v(x0, y1, 0.0, 1.0)]);
        self.upload_draw();
    }

    #[inline]
    pub fn tri(&mut self, a: (f32, f32), b: (f32, f32), c2: (f32, f32), c: [f32; 4]) {
        if self.verts.len() + 3 > MAX_VERTS * 4 {
            return;
        }
        self.verts.push(Vertex { x: a.0, y: a.1, u: 0.0, v: 0.0, c });
        self.verts.push(Vertex { x: b.0, y: b.1, u: 0.0, v: 0.0, c });
        self.verts.push(Vertex { x: c2.0, y: c2.1, u: 0.0, v: 0.0, c });
    }

    pub fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 4]) {
        self.tri((x0, y0), (x1, y0), (x1, y1), c);
        self.tri((x0, y0), (x1, y1), (x0, y1), c);
    }

    pub fn rect_outline(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, t: f32, c: [f32; 4]) {
        self.rect(x0, y0, x1, y0 + t, c);
        self.rect(x0, y1 - t, x1, y1, c);
        self.rect(x0, y0, x0 + t, y1, c);
        self.rect(x1 - t, y0, x1, y1, c);
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, seg: usize, c: [f32; 4]) {
        let step = std::f32::consts::TAU / seg as f32;
        for i in 0..seg {
            let a0 = i as f32 * step;
            let a1 = a0 + step;
            self.tri((cx, cy), (cx + a0.cos() * r, cy + a0.sin() * r), (cx + a1.cos() * r, cy + a1.sin() * r), c);
        }
    }

    pub fn ring(&mut self, cx: f32, cy: f32, r: f32, t: f32, seg: usize, c: [f32; 4]) {
        let step = std::f32::consts::TAU / seg as f32;
        let r2 = r - t;
        for i in 0..seg {
            let a0 = i as f32 * step;
            let a1 = a0 + step;
            let p0 = (cx + a0.cos() * r, cy + a0.sin() * r);
            let p1 = (cx + a1.cos() * r, cy + a1.sin() * r);
            let q0 = (cx + a0.cos() * r2, cy + a0.sin() * r2);
            let q1 = (cx + a1.cos() * r2, cy + a1.sin() * r2);
            self.tri(p0, p1, q1, c);
            self.tri(p0, q1, q0, c);
        }
    }

    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), t: f32, c: [f32; 4]) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = (dx * dx + dy * dy).sqrt().max(1e-5);
        let (nx, ny) = (-dy / l * t * 0.5, dx / l * t * 0.5);
        self.tri((a.0 + nx, a.1 + ny), (b.0 + nx, b.1 + ny), (b.0 - nx, b.1 - ny), c);
        self.tri((a.0 + nx, a.1 + ny), (b.0 - nx, b.1 - ny), (a.0 - nx, a.1 - ny), c);
    }

    // ------------------------------------------------------------ Direct2D
    pub fn d2d_begin(&mut self) -> bool {
        self.flush();
        unsafe {
            self.ctx.Flush();
        }
        if let Some(rt) = &self.d2d {
            unsafe {
                rt.BeginDraw();
                let u = self.ui;
                rt.SetTransform(&windows_numerics::Matrix3x2 { M11: u, M12: 0.0, M21: 0.0, M22: u, M31: 0.0, M32: 0.0 });
            }
            true
        } else {
            false
        }
    }

    pub fn d2d_end(&mut self) {
        if let Some(rt) = &self.d2d {
            unsafe {
                if let Err(e) = rt.EndDraw(None, None) {
                    self.warn_device("EndDraw", &e);
                    if e.code() == D2DERR_RECREATE_TARGET {
                        self.d2d = None;
                        if let Err(e) = self.create_targets() {
                            self.warn_device("create_targets", &e);
                        }
                    }
                }
            }
        }
    }

    fn set_brush(&self, c: [f32; 4]) -> Option<&ID2D1SolidColorBrush> {
        let b = self.brush.as_ref()?;
        unsafe {
            b.SetColor(&D2D1_COLOR_F { r: c[0], g: c[1], b: c[2], a: c[3] });
        }
        Some(b)
    }

    pub fn fill_rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 4]) {
        let (Some(rt), Some(b)) = (&self.d2d, self.set_brush(c)) else { return };
        unsafe {
            rt.FillRectangle(&D2D_RECT_F { left: x0, top: y0, right: x1, bottom: y1 }, b);
        }
    }

    pub fn stroke_rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: [f32; 4]) {
        let (Some(rt), Some(b)) = (&self.d2d, self.set_brush(c)) else { return };
        unsafe {
            rt.DrawRectangle(&D2D_RECT_F { left: x0, top: y0, right: x1, bottom: y1 }, b, w, None);
        }
    }

    pub fn fill_round(&self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, c: [f32; 4]) {
        let (Some(rt), Some(b)) = (&self.d2d, self.set_brush(c)) else { return };
        unsafe {
            rt.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: x0, top: y0, right: x1, bottom: y1 }, radiusX: r, radiusY: r }, b);
        }
    }

    pub fn stroke_round(&self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, w: f32, c: [f32; 4]) {
        let (Some(rt), Some(b)) = (&self.d2d, self.set_brush(c)) else { return };
        unsafe {
            rt.DrawRoundedRectangle(&D2D1_ROUNDED_RECT { rect: D2D_RECT_F { left: x0, top: y0, right: x1, bottom: y1 }, radiusX: r, radiusY: r }, b, w, None);
        }
    }

    pub fn fill_circle(&self, x: f32, y: f32, r: f32, c: [f32; 4]) {
        let (Some(rt), Some(b)) = (&self.d2d, self.set_brush(c)) else { return };
        unsafe {
            rt.FillEllipse(&D2D1_ELLIPSE { point: windows_numerics::Vector2 { X: x, Y: y }, radiusX: r, radiusY: r }, b);
        }
    }

    pub fn stroke_circle(&self, x: f32, y: f32, r: f32, w: f32, c: [f32; 4]) {
        let (Some(rt), Some(b)) = (&self.d2d, self.set_brush(c)) else { return };
        unsafe {
            rt.DrawEllipse(&D2D1_ELLIPSE { point: windows_numerics::Vector2 { X: x, Y: y }, radiusX: r, radiusY: r }, b, w, None);
        }
    }

    pub fn text(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, f: Font, c: [f32; 4]) {
        // 同じ文・書体・枠の配置は覚えておく（毎フレーム組み直すと重い）
        let key = (s.to_string(), f as u8, w.to_bits(), h.to_bits());
        let layout = match self.layout_cache.get(&key) {
            Some(l) => Some(l.clone()),
            None if w > 0.0 && h > 0.0 => {
                let wb: Vec<u16> = s.encode_utf16().collect();
                let l = unsafe { self.dwrite.CreateTextLayout(&wb, &self.fonts[f as usize], w, h) }.ok();
                if let Some(l) = &l {
                    if self.layout_cache.len() > 2048 {
                        self.layout_cache.clear();
                    }
                    self.layout_cache.insert(key, l.clone());
                }
                l
            }
            None => None,
        };
        if layout.is_none() {
            self.wbuf.clear();
            self.wbuf.extend(s.encode_utf16());
        }
        let (Some(rt), Some(b)) = (&self.d2d, self.set_brush(c)) else { return };
        unsafe {
            match &layout {
                Some(l) => rt.DrawTextLayout(windows_numerics::Vector2 { X: x, Y: y }, l, b, D2D1_DRAW_TEXT_OPTIONS_NONE),
                None => rt.DrawText(&self.wbuf, &self.fonts[f as usize], &D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h }, b, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL),
            }
        }
    }

    /// 言語が変わったら書体と窓の題名を作り直す
    pub fn reload_fonts(&mut self) {
        if let Ok(f) = make_fonts(&self.dwrite) {
            self.fonts = f;
        }
        self.measure_cache.clear();
        self.layout_cache.clear();
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowTextW(self.hwnd, &HSTRING::from(crate::lang::window_title()));
        }
    }

    /// 影付き文字
    pub fn text_shadow(&mut self, s: &str, x: f32, y: f32, w: f32, h: f32, f: Font, c: [f32; 4]) {
        self.text(s, x + 1.0, y + 1.0, w, h, f, [0.0, 0.0, 0.0, c[3] * 0.85]);
        self.text(s, x, y, w, h, f, c);
    }

    /// テキスト幅の計測
    /// 計測を覚えておく（同じ文は測り直さない）。幅 0 は「1行の幅」を表す
    fn cached(&mut self, s: &str, f: Font, width: f32, calc: impl FnOnce(&mut Self) -> f32) -> f32 {
        let key = (s.to_string(), f as u8, width.to_bits());
        if let Some(&v) = self.measure_cache.get(&key) {
            return v;
        }
        let v = calc(self);
        if self.measure_cache.len() > 4096 {
            self.measure_cache.clear();
        }
        self.measure_cache.insert(key, v);
        v
    }

    pub fn measure(&mut self, s: &str, f: Font) -> f32 {
        self.cached(s, f, 0.0, |g| g.measure_raw(s, f))
    }

    pub fn measure_h(&mut self, s: &str, f: Font, width: f32) -> f32 {
        self.cached(s, f, width, |g| g.measure_h_raw(s, f, width))
    }

    fn measure_raw(&mut self, s: &str, f: Font) -> f32 {
        let wb: Vec<u16> = s.encode_utf16().collect();
        unsafe {
            if let Ok(layout) = self.dwrite.CreateTextLayout(&wb, &self.fonts[f as usize], 4000.0, 200.0) {
                let mut m = DWRITE_TEXT_METRICS::default();
                if layout.GetMetrics(&mut m).is_ok() {
                    return m.widthIncludingTrailingWhitespace;
                }
            }
        }
        s.chars().count() as f32 * 14.0
    }

    /// 幅 width で折り返したときの文章の高さ
    fn measure_h_raw(&mut self, s: &str, f: Font, width: f32) -> f32 {
        let wb: Vec<u16> = s.encode_utf16().collect();
        unsafe {
            if let Ok(layout) = self.dwrite.CreateTextLayout(&wb, &self.fonts[f as usize], width, 4000.0) {
                let mut m = DWRITE_TEXT_METRICS::default();
                if layout.GetMetrics(&mut m).is_ok() {
                    return m.height;
                }
            }
        }
        20.0 * (1 + s.chars().count() / 24) as f32
    }

    pub fn create_bitmap(&self, w: u32, h: u32, data: &[u32]) -> Option<ID2D1Bitmap> {
        let rt = self.d2d.as_ref()?;
        unsafe {
            let props = D2D1_BITMAP_PROPERTIES { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED }, dpiX: 96.0, dpiY: 96.0 };
            rt.CreateBitmap(D2D_SIZE_U { width: w, height: h }, Some(data.as_ptr() as _), w * 4, &props).ok()
        }
    }

    pub fn update_bitmap(&self, bmp: &ID2D1Bitmap, w: u32, data: &[u32]) {
        unsafe {
            let _ = bmp.CopyFromMemory(None, data.as_ptr() as _, w * 4);
        }
    }

    pub fn draw_bitmap(&self, bmp: &ID2D1Bitmap, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.draw_bitmap_alpha(bmp, x0, y0, x1, y1, 1.0);
    }

    pub fn draw_bitmap_alpha(&self, bmp: &ID2D1Bitmap, x0: f32, y0: f32, x1: f32, y1: f32, alpha: f32) {
        let Some(rt) = &self.d2d else { return };
        unsafe {
            rt.DrawBitmap(bmp, Some(&D2D_RECT_F { left: x0, top: y0, right: x1, bottom: y1 }), alpha, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
        }
    }

    /// バックバッファを BMP に保存（自動テスト用）
    pub fn save_backbuffer(&self, path: &str) -> Result<()> {
        unsafe {
            let bb: ID3D11Texture2D = self.swapchain.GetBuffer(0)?;
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            bb.GetDesc(&mut desc);
            desc.Usage = D3D11_USAGE_STAGING;
            desc.BindFlags = 0;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            desc.MiscFlags = 0;
            let mut st = None;
            self.device.CreateTexture2D(&desc, None, Some(&mut st))?;
            let st = st.unwrap();
            self.ctx.CopyResource(&st, &bb);
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            self.ctx.Map(&st, 0, D3D11_MAP_READ, 0, Some(&mut m))?;
            let (w, h) = (desc.Width as usize, desc.Height as usize);
            let mut out = Vec::with_capacity(54 + w * h * 4);
            let size = (54 + w * h * 4) as u32;
            out.extend_from_slice(b"BM");
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&[0, 0, 0, 0]);
            out.extend_from_slice(&54u32.to_le_bytes());
            out.extend_from_slice(&40u32.to_le_bytes());
            out.extend_from_slice(&(w as i32).to_le_bytes());
            out.extend_from_slice(&(-(h as i32)).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&32u16.to_le_bytes());
            out.extend_from_slice(&[0u8; 24]);
            for y in 0..h {
                let row = std::slice::from_raw_parts((m.pData as *const u8).add(y * m.RowPitch as usize), w * 4);
                out.extend_from_slice(row);
            }
            self.ctx.Unmap(&st, 0);
            let _ = std::fs::write(path, out);
        }
        Ok(())
    }

    pub fn present(&mut self) {
        unsafe {
            if let Err(e) = self.swapchain.Present(1, DXGI_PRESENT(0)).ok() {
                self.warn_device("Present", &e);
            }
        }
    }

    /// 描画の失敗をログに書く（デバイス消失ならその理由も）
    fn warn_device(&self, what: &'static str, e: &Error) {
        let reason = unsafe { self.device.GetDeviceRemovedReason() };
        crate::crashlog::warn_limited(what, &format!("{} デバイス: {:?}", e, reason));
    }
}
