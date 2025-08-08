use bevy_ecs::prelude::*;
use anyhow::Result;
use std::{
    collections::HashMap, fs::{self, metadata, File}, io::Read, path::{Path, PathBuf}
};
use tracing::info;

pub enum Asset {
    Image(Vec<u8>), 
    Shader(String)
}

impl Asset {
    pub fn from_file(path: &Path, mut file: File) -> Option<Self> {
        let metadata = metadata(path)
            .expect(format!("Error loading metadata for file").as_str());
        /*
            Check different matches here         
         */
        let mut asset: Option<Self> = None;

        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext == "png" {
                let mut buffer = vec![0; metadata.len() as usize];
                file.read(&mut buffer)
                    .expect("Error reading texture for file");

                asset = Some(Asset::Image(buffer));
            } else if ext == "wgsl" {
                let mut shader_source = String::new();
                file.read_to_string(&mut shader_source)
                    .expect("Error reading shader {}");

                asset = Some(Asset::Shader(shader_source));
            }
    
        }

        return asset;
    }
}

#[derive(Resource)]
pub struct AssetCache {
    pub map: HashMap<String, Asset>
}

impl AssetCache {
    pub fn init(dir: &str) -> Result<Self> {
        let mut map : HashMap<String, Asset> = HashMap::new();

        let mut paths = vec![PathBuf::from(dir)];
        while !paths.is_empty() {
            let path = paths.pop().unwrap();
            
            let entries = fs::read_dir(path)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    paths.push(path);
                } else {
                    let file = File::open(&path)?;

                    if let Some(asset) = Asset::from_file(&path, file) {
                        let new_name= path
                            .clone()
                            .components() 
                            .skip(1)
                            .collect::<PathBuf>() 
                            .to_string_lossy()
                            .to_string();
                        
                        info!("Inserting asset {}", new_name);
                        map.insert(new_name, asset);
                    }

                }
            }
        }
        Ok(AssetCache { map: map })
    }


    pub fn get_shader(&self, name: &str) -> Option<&String> {
        if let Some(asset) = self.map.get(name) {
            match asset {
                Asset::Shader(src) => {
                    return Some(src); 
                }
                _ => {}
            }

        }
        return None; 
    } 

    pub fn get_image(&self, name: &str) -> Option<&[u8]> {
        if let Some(asset) = self.map.get(name) {
            match asset {
                Asset::Image(tex) => {
                    return Some(tex);
                }
                _ => {}
            }
        }
        return None; 
    }
}
