# Great War visual assets

Generated with the built-in `image_gen.imagegen` tool. Final generation prompts are in [prompts.json](prompts.json). The equipment and buildings atlases received a second ImageGen pass to remove their backdrops. Original selected atlases are retained here; PNG sprites and premultiplied BGRA files are the production assets.

## Import

Run from the repository root with Python and Pillow installed:

```sh
python tools/import_generated_art.py assets/sources/infantry-atlas.png infantry
python tools/import_generated_art.py assets/sources/equipment-atlas.png equipment
python tools/import_generated_art.py assets/sources/buildings-atlas.png buildings
python tools/import_generated_art.py assets/sources/late-atlas.png late
```

The importer preserves alpha and the renderer's dimensions (160px units, 192px buildings). The procedural `tools/gen_art.py` is retained as a legacy tool and requires `--legacy-overwrite` to overwrite these assets.

## Historical direction

- Twenty national infantry variants follow the `geo::NATIONS` order, including neutral countries. Early uniforms generally represent 1914; the US represents its 1917 entry.
- Germany and Britain change to steel-helmet field dress in 1916; France and Belgium change in mid-1915. These are approximate visual transitions, not an assertion that all regiments changed simultaneously.
- British tanks use a rhomboid Mark IV silhouette. French and American tanks use the Renault FT silhouette. Other equipment uses representative period silhouettes shared between countries: national models for every gun, cavalry regiment and ship are not simulated.
- Towns, capitals, farms and industrial ports represent European architecture. The trench position replaces a fortress silhouette. Geography props and the game's supernatural endgame assets retain their existing artwork.
- The UI uses ink blue, field grey, ivory type and restrained brass drafting rules. Bamboo slips, carved stone, lacquered timber and ornate crest frames have been replaced.

Uniform reference: [National Army Museum: Uniform and Equipment of 1914](https://ww1.nam.ac.uk/937/news/uniform-equipment-1914/) and [First World War in Five Objects](https://ww1.nam.ac.uk/1480/news/first-world-war-five-objects/). Generated artwork is a readable game interpretation; small insignia and rare uniform details are not museum reconstructions.
