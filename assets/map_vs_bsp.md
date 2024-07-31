# Pros and cons of using BSPs over .map files
### Pros
- Optimised Meshes: You don't have to manually set non-visible faces' texture to empty, *and* non-visible *parts* of faces will also be cut out, further reducing overdraw.
- Lightmaps: You get pre-computed lighting and global illumination for free! Just as long as you're fine foregoing sharp shadows
- Embedded Textures: You can mix and match using loose files and embedded textures using WADs. The only caveat is that WADs must use the palette defined in your assets folder. (See TrenchBroomConfig::texture_pallette docs)
- Vis data

### Cons
- Higher iteration time: Compiling is an extra step before you get to play on your map, though with TrenchBroom's inbuilt compiler utility, it's not that bad.
- Lightmaps are static: If your scene is highly dynamic, you probably won't want lightmaps, or 

`bevy_trenchbroom` supports loading BSP2 files with

NOTE: At the time of writing, the latest version of ericw-tools (0.18.1) has a bug relating to writing loose textures into the BSP, so if it hasn't updated yet, [use the master branch](https://github.com/ericwa/ericw-tools/tree/brushbsp?tab=readme-ov-file#compiling).