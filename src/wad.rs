use crate::*;

pub fn create_wad(textures: HashMap<String, image::RgbImage>) -> Vec<u8> {
    // typedef struct
    // {
    //     long offset;                 // Position of the entry in WAD
    //     long dsize;                  // Size of the entry in WAD file
    //     long size;                   // Size of the entry in memory
    //     char type;                   // type of entry
    //     char cmprs;                  // Compression. 0 if none.
    //     short dummy;                 // Not used
    //     char name[16];               // 1 to 16 characters, '\0'-padded
    // } wadentry_t;
    const WAD_ENTRY_SIZE: usize = 4 + 4 + 4 + 1 + 1 + 2 + 16;
    
    let mut data = Vec::new();

    // Magic number
    data.extend(b"WAD2");

    // Currently, each image will have it's own palette
    // Number of directory entries
    let num_entries = textures.len() * 2;
    data.extend((num_entries as u32).to_le_bytes());
    
    let directory_start = data.len() + 4;
    let directory_size = num_entries * WAD_ENTRY_SIZE;
    
    // File offset of directory (right after this)
    data.extend((directory_start as u32).to_le_bytes());

    // Since we know how much space the directory is going to take up, allocate it first, allowing to dynamic lump allocation
    data.extend(repeat_n(0, directory_size));
    
    for (i, (name, image)) in textures.into_iter().enumerate() {
        if name.len() > 16 {
            warn!("Texture \"{name}\" has a name more than 16 characters long! It will be cut off when writing the WAD.");
        }
        let entry_start = directory_start + i * WAD_ENTRY_SIZE * 2;
        let name: [u8; 16] = name.into_bytes().into_iter().take(16).pad_using(16, |_| 0).collect_vec().try_into().unwrap();
        
        let (indexed_image, palette) = index_image(&image);

        let mut entry = Vec::new();

        // Palette first
        
        // Position of the entry in the WAD
        entry.extend((data.len() as u32).to_le_bytes());
        // Size in the wad file
        entry.extend((palette.len() as u32).to_le_bytes());
        // Size in memory? i'll just put the same as above for now
        entry.extend((palette.len() as u32).to_le_bytes());
        // Color palette marker byte
        entry.push('@' as u8);
        // No compression
        entry.push(0);
        // Dummy short
        entry.extend(0u16.to_le_bytes());
        entry.extend(&name);

        data.extend(&palette);

        // Now time for the actual image

        // Position of the entry in the WAD
        entry.extend((data.len() as u32).to_le_bytes());
        // Size in the wad file
        entry.extend((indexed_image.len() as u32).to_le_bytes());
        // Size in memory? i'll just put the same as above for now
        entry.extend((indexed_image.len() as u32).to_le_bytes());
        // Status bar picture marker byte?
        entry.push('h' as u8);
        // No compression
        entry.push(0);
        // Dummy short
        entry.extend(0u16.to_le_bytes());
        entry.extend(&name);

        data.extend(indexed_image);

        assert!(entry.len() == WAD_ENTRY_SIZE * 2);
        
        for (i, byte) in entry.into_iter().enumerate() {
            data[entry_start + i] = byte;
        }
    }

    data
}

const PALETTE_SIZE: usize = 3*256;

fn index_image(image: &image::RgbImage) -> (Vec<u8>, [u8; PALETTE_SIZE]) {
    let (palette, indexed) = exoquant::convert_to_indexed(
        &image.pixels().map(|pixel| exoquant::Color::new(pixel.0[0], pixel.0[1], pixel.0[2], 0)).collect_vec(),
        image.width() as usize,
        256,
        &exoquant::optimizer::KMeans,
        &exoquant::ditherer::FloydSteinberg::new()
    );
    
    (indexed, palette.into_iter().map(|color| [color.r, color.g, color.b]).flatten().collect_vec().try_into().unwrap())
}