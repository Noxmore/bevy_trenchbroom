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
        let name: [u8; 16] = name.into_bytes().into_iter().take(16).pad_using(16, |_| 0).collect_vec().try_into().unwrap();
        
        let mut current_entry_byte = directory_start + i * WAD_ENTRY_SIZE * 2;
        macro_rules! entry_extend {($bytes:expr) => {
            for byte in $bytes {
                data[current_entry_byte] = byte;
                current_entry_byte += 1;
            }
        };}

        // 
        
        // Position of the entry in the WAD
        entry_extend!((data.len() as u32).to_le_bytes());
        
        // data.extend(iter);
    }

    data
}