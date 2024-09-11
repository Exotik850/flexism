use std::path::Path;

use anyhow::Context;

use flexism::plugin::{key_value::Host as KVHost, plugin_register::Host as PRHost};
use wasmtime::{
    component::{Component, Linker},
    Config, Engine, Store,
};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

wasmtime::component::bindgen!({
  path: "../flexism-plugin/wit/plugin.wit",
  world: "plugin",
  async: true,
});

struct PluginHost {
    map: std::collections::HashMap<String, Vec<u8>>,
    table: ResourceTable,
    ctx: WasiCtx,
}

impl PluginHost {
    fn new() -> Self {
        let table = ResourceTable::new();
        let ctx = WasiCtxBuilder::new().inherit_stdio().build();
        Self {
            map: std::collections::HashMap::new(),
            table,
            ctx,
        }
    }
}

#[async_trait::async_trait]
impl KVHost for PluginHost {
    async fn get(&mut self, key: String) -> Option<Vec<u8>> {
        self.map.get(&key).cloned()
    }

    async fn set(&mut self, key: String, value: Vec<u8>) -> Result<(), String> {
        self.map.insert(key, value);
        Ok(())
    }

    async fn delete(&mut self, key: String) -> Result<(), String> {
        self.map.remove(&key);
        Ok(())
    }
}

#[async_trait::async_trait]
impl PRHost for PluginHost {
    async fn register_event(&mut self, event: String) -> bool {
        println!("Event: {}", event);
        true
    }

    async fn emit_event(&mut self, event: String, data: Vec<u8>) -> Result<(), String> {
        println!("Emit: {event} <-> {data:?}");
        Ok(())
    }

    async fn toggle_event(&mut self, event: String) -> Option<bool> {
        println!("Toggle: {}", event);
        Some(true)
    }
}

// struct ServerWasiView {
//     table: ResourceTable,
//     ctx: WasiCtx,
// }

// impl ServerWasiView {
//     fn new() -> Self {
//         let table = ResourceTable::new();
//         let ctx = WasiCtxBuilder::new().inherit_stdio().build();

//         Self { table, ctx }
//     }
// }

impl WasiView for PluginHost {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

async fn run() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("Expected path to component file")?;
    let mut config = Config::default();
    config.wasm_component_model(true);
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let mut linker = Linker::new(&engine);

    // Add the command world (aka WASI CLI) to the linker
    wasmtime_wasi::add_to_linker_async(&mut linker)?;
    Plugin::add_to_linker(&mut linker, |state| state)?;

    let wasi_view = PluginHost::new();
    let mut store = Store::new(&engine, wasi_view);

    let instance = load_plugin(engine, path, &mut store, linker).await?;
    let impls = instance.flexism_plugin_plugin_impl();

    let name = impls.call_name(&mut store).await?;

    instance.unload(&mut store).await?;

    println!("Name: {}", name);

    Ok(())
}

impl Plugin {
    async fn unload(&self, store: &mut Store<PluginHost>) -> anyhow::Result<()> {
        if let Err(e) = self
            .flexism_plugin_plugin_impl()
            .call_unload(store)
            .await?
        {
            anyhow::bail!(e)
        };
        Ok(())
    }
}

async fn load_plugin(
    engine: Engine,
    path: String,
    store: &mut Store<PluginHost>,
    linker: Linker<PluginHost>,
) -> Result<Plugin, anyhow::Error> {
    let component = Component::from_file(&engine, path).context("Component file not found")?;
    let instance = Plugin::instantiate_async(&mut *store, &component, &linker).await?;
    let impls = instance.flexism_plugin_plugin_impl();
    if let Err(e) = impls.call_load(&mut *store).await? {
        anyhow::bail!(e)
    };
    Ok(instance)
}

fn main() -> anyhow::Result<()> {
    async_std::task::block_on(run())
}
