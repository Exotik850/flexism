use exports::flexism::plugin::plugin_impl::Guest as ImplGuest;
// use exports::flexism::plugin::pl::Guest as MetaGuest;
use flexism_plugin::{
    export,
    exports::{self, flexism::plugin::plugin_meta::Guest as MetaGuest},
};
struct Plugin;

impl MetaGuest for Plugin {
    fn name() -> String {
        "Test".to_string()
    }

    fn version() -> String {
        "0.1.0".to_string()
    }

    fn description() -> String {
        "Test plugin".to_string()
    }

    fn requires() -> Vec<String> {
        vec![]
    }
}

impl ImplGuest for Plugin {
    fn load() -> Result<(), String> {
        println!("Loading plugin");
        let read = std::fs::read_dir("/assets").map_err(|e| e.to_string())?;
        for file in read {
            let file = file.map_err(|e| e.to_string())?;
            println!("Found file: {:?}", file.file_name());
        }
        Ok(())
    }

    fn unload() -> Result<(), String> {
        println!("Unloading plugin");
        Ok(())
    }

    fn enable() -> Result<(), String> {
        todo!()
    }

    fn disable() -> Result<(), String> {
        todo!()
    }

    fn on_event(event: String, data: Vec<u8>) -> Result<(), String> {
        todo!()
    }
}

export!(Plugin);
