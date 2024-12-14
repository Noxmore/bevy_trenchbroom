# Pros and cons of using BSPs over .map files
### Pros
- You don't have to manually set non-visible faces' texture to empty. Non-visible parts of faces will also be cut out, further reducing overdraw.
- You get pre-computed lighting and global illumination for free! The lightmap resolution is 16 TrenchBroom units per luxel, so if you want sharp shadows, you can also spawn normal lights along with the lightmap.
- You can mix and match using loose files and embedded textures using WADs. The only caveat is that WADs must use the palette defined in your assets folder. (See TrenchBroomConfig::texture_pallette docs)
- Vis data: TODO

### Cons
- Higher iteration time: Compiling is an extra step before you get to play on your map, though with TrenchBroom's inbuilt compiler utility, it's not that bad.
- Lightmaps are static: If your scene is highly dynamic, you probably won't want lightmaps. (though, you can always compile without lighting data)

TODO list is not final

NOTE: At the time of writing, the latest version of ericw-tools (0.18.1) has a bug relating to writing loose textures into the BSP, so if it hasn't updated yet, [use the master branch](https://github.com/ericwa/ericw-tools/tree/brushbsp?tab=readme-ov-file#compiling).