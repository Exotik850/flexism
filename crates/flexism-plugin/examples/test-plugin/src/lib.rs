use exports::flexism::plugin::plugin_impl::Guest;
use flexism_plugin::{export, exports};
struct Plugin;

impl Guest for Plugin {
    fn name() -> String {
        "Test".to_string()
    }

    fn version() -> String {
        "0.1.0".to_string()
    }

    fn description() -> String {
        "Test plugin".to_string()
    }

    fn load() -> Result<(), String> {
        println!("Loading plugin");
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
