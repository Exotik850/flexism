use std::{borrow::Cow, collections::HashMap, path::Path};

use anyhow::Context;

use async_std::channel::Sender;
use flexism::plugin::{key_value::Host as KVHost, plugin_register::Host as PRHost};
use wasmtime::{
    component::{Component, Linker},
    AsContextMut, Config, Engine, Store,
};
use wasmtime_wasi::{DirPerms, FilePerms, ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

wasmtime::component::bindgen!({
  path: "../flexism-plugin/wit/plugin.wit",
  world: "plugin",
  async: true,
});

#[derive(Clone, Eq, Hash, PartialEq)]
struct PluginId<'a>(Cow<'a, str>);

struct Event {
    name: String,
    data: Vec<u8>,
}

struct PluginManager<'a> {
    plugins: HashMap<PluginId<'a>, Plugin>,
    events: HashMap<String, Vec<PluginId<'a>>>,
    current_plugin: Option<PluginId<'a>>,
    thread: std::thread::JoinHandle<()>,
}

struct PluginWorld {
    map: std::collections::HashMap<String, Vec<u8>>,
    table: ResourceTable,
    ctx: WasiCtx,
    event_channel: Sender<Event>, // pub store: Store<PluginHost>,
}

impl PluginWorld {
    fn new(event_channel: Sender<Event>) -> anyhow::Result<Self> {
        let cwd = std::env::current_dir()?;
        let assets_path = cwd.join("plugin-assets");
        if !assets_path.exists() {
            std::fs::create_dir_all(&assets_path)?;
        }
        let table = ResourceTable::new();
        let ctx = WasiCtxBuilder::new()
            .inherit_stdout()
            .preopened_dir(assets_path, "assets", DirPerms::all(), FilePerms::all())?
            .build();
        Ok(Self {
            map: std::collections::HashMap::new(),
            // plugins: HashMap::new(),
            // events: HashMap::new(),
            // current_plugin: None,
            event_channel,
            table,
            ctx,
        })
    }
}

#[async_trait::async_trait]
impl KVHost for PluginWorld {
    async fn get(&mut self, key: String) -> Option<Vec<u8>> {
        self.map.get(&key).cloned()
    }

    async fn set(&mut self, key: String, value: Vec<u8>) {
        self.map.insert(key, value);
    }

    async fn delete(&mut self, key: String) -> bool {
        let res = self.map.remove(&key);
        res.is_some()
    }
}

#[async_trait::async_trait]
impl PRHost for PluginWorld {
    async fn register_event(&mut self, event: String) -> bool {
        // let Some(plugin_id) = self.current_plugin.as_ref() else {
        //     return false;
        // };
        // self.events
        //     .entry(event)
        //     .or_insert_with(Vec::new)
        //     .push(plugin_id.clone());
        true
    }

    async fn emit_event(&mut self, event: String, data: Vec<u8>) -> Result<(), String> {
        self.event_channel.send(Event { name: event, data }).await;
        Ok(())
    }

    async fn toggle_event(&mut self, event: String) -> Option<bool> {
        // TODO make this disable event
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

impl WasiView for PluginWorld {
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

    let (tx, rx) = async_std::channel::bounded(100);
    let wasi_view = PluginWorld::new(tx)?;
    let mut store = Store::new(&engine, wasi_view);

    let instance = load_plugin(engine, path, &mut store, linker).await?;
    let impls = instance.flexism_plugin_plugin_meta();

    let name = impls.call_name(&mut store).await?;

    instance.unload(&mut store).await?;

    println!("Name: {}", name);

    Ok(())
}

impl Plugin {
    async fn unload(&self, store: &mut Store<PluginWorld>) -> anyhow::Result<()> {
        if let Err(e) = self.flexism_plugin_plugin_impl().call_unload(store).await? {
            anyhow::bail!(e)
        };
        Ok(())
    }
}

async fn load_plugin(
    engine: Engine,
    path: String,
    store: &mut Store<PluginWorld>,
    linker: Linker<PluginWorld>,
) -> anyhow::Result<Plugin> {
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
