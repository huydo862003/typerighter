use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

use futures::future::{AbortHandle, Abortable};
use jsonrpsee::wasm_client::{Client as WasmClient, WasmClientBuilder};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::rpc::contract::{
  CANCELLED_ERROR_CODE, TdBuildRpcClient, TdBuiltResource, TdDiagnosticReport, TdFilePath,
  TdFormatResult, TdSchemaInfo, TdSidebarItem, TdSiteConfig,
};

#[wasm_bindgen(js_name = "RPC_CANCELLED_CODE")]
pub fn rpc_cancelled_code() -> i32 {
  CANCELLED_ERROR_CODE
}

#[wasm_bindgen]
pub struct RpcClient {
  inner: Arc<WasmClient>,
  content_changed: RefCell<ListenerSlot>,
  content_created: RefCell<ListenerSlot>,
  content_deleted: RefCell<ListenerSlot>,
  schema_changed: RefCell<ListenerSlot>,
  schema_created: RefCell<ListenerSlot>,
  schema_deleted: RefCell<ListenerSlot>,
  config_changed: RefCell<ListenerSlot>,
  disconnect: RefCell<ListenerSlot>,
}

impl RpcClient {
  fn abort_all_subscriptions(&self) {
    self.content_changed.borrow_mut().abort();
    self.content_created.borrow_mut().abort();
    self.content_deleted.borrow_mut().abort();
    self.schema_changed.borrow_mut().abort();
    self.schema_created.borrow_mut().abort();
    self.schema_deleted.borrow_mut().abort();
    self.config_changed.borrow_mut().abort();
    self.disconnect.borrow_mut().abort();
  }
}

/// Call all registered callbacks with the given arguments
fn notify_all(callbacks: &RefCell<Vec<js_sys::Function>>, args: &[JsValue]) {
  for callback in callbacks.borrow().iter() {
    let _ = callback.apply(&JsValue::NULL, &js_sys::Array::from_iter(args));
  }
}

#[wasm_bindgen]
impl RpcClient {
  #[allow(unused_variables)]
  #[wasm_bindgen(static_method_of = RpcClient)]
  pub async fn connect(addr: String, port: u16) -> Result<RpcClient, JsValue> {
    let url = format!("ws://{addr}:{port}");
    let inner = WasmClientBuilder::default()
      .request_timeout(std::time::Duration::from_secs(120))
      .build(&url)
      .await
      .map_err(rpc_err)?;
    Ok(RpcClient {
      inner: Arc::new(inner),
      content_changed: RefCell::new(ListenerSlot::new()),
      content_created: RefCell::new(ListenerSlot::new()),
      content_deleted: RefCell::new(ListenerSlot::new()),
      schema_changed: RefCell::new(ListenerSlot::new()),
      schema_created: RefCell::new(ListenerSlot::new()),
      schema_deleted: RefCell::new(ListenerSlot::new()),
      config_changed: RefCell::new(ListenerSlot::new()),
      disconnect: RefCell::new(ListenerSlot::new()),
    })
  }

  pub fn close(&self) {
    self.abort_all_subscriptions();
  }

  #[wasm_bindgen(js_name = "requestFile")]
  pub async fn request_file(&self, path: String) -> Result<TdBuiltResource, JsValue> {
    <WasmClient as TdBuildRpcClient<(), ()>>::request_file(&*self.inner, TdFilePath(path))
      .await
      .map_err(rpc_err)
  }

  #[wasm_bindgen(js_name = "requestFiles")]
  pub async fn request_files(&self, paths: Vec<String>) -> Result<Vec<TdBuiltResource>, JsValue> {
    let file_paths = paths.into_iter().map(TdFilePath).collect();
    <WasmClient as TdBuildRpcClient<(), ()>>::request_files(&*self.inner, file_paths)
      .await
      .map_err(rpc_err)
  }

  #[wasm_bindgen(js_name = "listVault")]
  pub async fn list_vault(&self) -> Result<Vec<String>, JsValue> {
    <WasmClient as TdBuildRpcClient<(), ()>>::list_vault(&*self.inner)
      .await
      .map_err(rpc_err)
  }

  #[wasm_bindgen(js_name = "getVersion")]
  pub async fn get_version(&self) -> Result<String, JsValue> {
    <WasmClient as TdBuildRpcClient<(), ()>>::get_version(&*self.inner)
      .await
      .map_err(rpc_err)
  }

  #[wasm_bindgen(js_name = "getConfig")]
  pub async fn get_config(&self) -> Result<TdSiteConfig, JsValue> {
    <WasmClient as TdBuildRpcClient<(), ()>>::get_config(&*self.inner)
      .await
      .map_err(rpc_err)
  }

  #[wasm_bindgen(js_name = "listFilesGroupedBySchema")]
  pub async fn list_files_grouped_by_schema(&self) -> Result<JsValue, JsValue> {
    let result =
      <WasmClient as TdBuildRpcClient<(), ()>>::list_files_grouped_by_schema(&*self.inner)
        .await
        .map_err(rpc_err)?;
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
  }

  #[wasm_bindgen(js_name = "listSidebar")]
  pub async fn list_sidebar(&self) -> Result<Vec<TdSidebarItem>, JsValue> {
    <WasmClient as TdBuildRpcClient<(), ()>>::list_sidebar(&*self.inner)
      .await
      .map_err(rpc_err)
  }

  #[wasm_bindgen(js_name = "listSchemas")]
  pub async fn list_schemas(&self) -> Result<Vec<String>, JsValue> {
    <WasmClient as TdBuildRpcClient<(), ()>>::list_schemas(&*self.inner)
      .await
      .map_err(rpc_err)
  }

  #[wasm_bindgen(js_name = "getSchema")]
  pub async fn get_schema(&self, schema: String) -> Result<TdSchemaInfo, JsValue> {
    <WasmClient as TdBuildRpcClient<(), ()>>::get_schema(&*self.inner, schema)
      .await
      .map_err(rpc_err)
  }

  #[wasm_bindgen(js_name = "checkVault")]
  pub async fn check_vault(&self) -> Result<TdDiagnosticReport, JsValue> {
    <WasmClient as TdBuildRpcClient<(), ()>>::check_vault(&*self.inner)
      .await
      .map_err(rpc_err)
  }

  #[wasm_bindgen(js_name = "formatFile")]
  pub async fn format_file(&self, path: String) -> Result<TdFormatResult, JsValue> {
    <WasmClient as TdBuildRpcClient<(), ()>>::format_file(&*self.inner, TdFilePath(path))
      .await
      .map_err(rpc_err)
  }

  #[wasm_bindgen(js_name = "onContentChanged")]
  pub fn on_content_changed(&self, callback: js_sys::Function) {
    let client = Arc::clone(&self.inner);
    self
      .content_changed
      .borrow_mut()
      .add(callback, |callbacks| async move {
        let Ok(mut sub) =
          <WasmClient as TdBuildRpcClient<(), ()>>::subscribe_content_changed(&*client).await
        else {
          return;
        };
        while let Some(Ok(notif)) = sub.next().await {
          notify_all(&callbacks, &[notif.into()]);
        }
      });
  }

  #[wasm_bindgen(js_name = "onContentCreated")]
  pub fn on_content_created(&self, callback: js_sys::Function) {
    let client = Arc::clone(&self.inner);
    self
      .content_created
      .borrow_mut()
      .add(callback, |callbacks| async move {
        let Ok(mut sub) =
          <WasmClient as TdBuildRpcClient<(), ()>>::subscribe_content_created(&*client).await
        else {
          return;
        };
        while let Some(Ok(notif)) = sub.next().await {
          notify_all(&callbacks, &[notif.into()]);
        }
      });
  }

  #[wasm_bindgen(js_name = "onContentDeleted")]
  pub fn on_content_deleted(&self, callback: js_sys::Function) {
    let client = Arc::clone(&self.inner);
    self
      .content_deleted
      .borrow_mut()
      .add(callback, |callbacks| async move {
        let Ok(mut sub) =
          <WasmClient as TdBuildRpcClient<(), ()>>::subscribe_content_deleted(&*client).await
        else {
          return;
        };
        while let Some(Ok(notif)) = sub.next().await {
          notify_all(&callbacks, &[notif.into()]);
        }
      });
  }

  #[wasm_bindgen(js_name = "onSchemaChanged")]
  pub fn on_schema_changed(&self, callback: js_sys::Function) {
    let client = Arc::clone(&self.inner);
    self
      .schema_changed
      .borrow_mut()
      .add(callback, |callbacks| async move {
        let Ok(mut sub) =
          <WasmClient as TdBuildRpcClient<(), ()>>::subscribe_schema_changed(&*client).await
        else {
          return;
        };
        while let Some(Ok(notif)) = sub.next().await {
          notify_all(&callbacks, &[notif.into()]);
        }
      });
  }

  #[wasm_bindgen(js_name = "onSchemaCreated")]
  pub fn on_schema_created(&self, callback: js_sys::Function) {
    let client = Arc::clone(&self.inner);
    self
      .schema_created
      .borrow_mut()
      .add(callback, |callbacks| async move {
        let Ok(mut sub) =
          <WasmClient as TdBuildRpcClient<(), ()>>::subscribe_schema_created(&*client).await
        else {
          return;
        };
        while let Some(Ok(notif)) = sub.next().await {
          notify_all(&callbacks, &[notif.into()]);
        }
      });
  }

  #[wasm_bindgen(js_name = "onSchemaDeleted")]
  pub fn on_schema_deleted(&self, callback: js_sys::Function) {
    let client = Arc::clone(&self.inner);
    self
      .schema_deleted
      .borrow_mut()
      .add(callback, |callbacks| async move {
        let Ok(mut sub) =
          <WasmClient as TdBuildRpcClient<(), ()>>::subscribe_schema_deleted(&*client).await
        else {
          return;
        };
        while let Some(Ok(notif)) = sub.next().await {
          notify_all(&callbacks, &[notif.into()]);
        }
      });
  }

  #[wasm_bindgen(js_name = "onConfigChanged")]
  pub fn on_config_changed(&self, callback: js_sys::Function) {
    let client = Arc::clone(&self.inner);
    self
      .config_changed
      .borrow_mut()
      .add(callback, |callbacks| async move {
        let Ok(mut sub) =
          <WasmClient as TdBuildRpcClient<(), ()>>::subscribe_config_changed(&*client).await
        else {
          return;
        };
        while let Some(Ok(notif)) = sub.next().await {
          notify_all(&callbacks, &[notif.into()]);
        }
      });
  }

  #[wasm_bindgen(js_name = "onDisconnect")]
  pub fn on_disconnect(&self, callback: js_sys::Function) {
    let client = Arc::clone(&self.inner);
    self
      .disconnect
      .borrow_mut()
      .add(callback, |callbacks| async move {
        client.on_disconnect().await;
        notify_all(&callbacks, &[]);
      });
  }
}

/// A subscription slot that supports multiple JS callbacks
struct ListenerSlot {
  callbacks: Rc<RefCell<Vec<js_sys::Function>>>,
  abort: Option<AbortHandle>,
}

impl ListenerSlot {
  fn new() -> Self {
    Self {
      callbacks: Rc::new(RefCell::new(Vec::new())),
      abort: None,
    }
  }

  /// Add a callback
  /// If this is the first, start the subscription task
  fn add<Fut>(
    &mut self,
    callback: js_sys::Function,
    start_sub: impl FnOnce(Rc<RefCell<Vec<js_sys::Function>>>) -> Fut + 'static,
  ) where
    Fut: Future<Output = ()> + 'static,
  {
    self.callbacks.borrow_mut().push(callback);

    // Only start the subscription on the first listener
    if self.abort.is_some() {
      return;
    }

    let (handle, reg) = AbortHandle::new_pair();
    self.abort = Some(handle);
    let callbacks = Rc::clone(&self.callbacks);

    spawn_local(async move {
      let _ = Abortable::new(start_sub(callbacks), reg).await;
    });
  }

  fn abort(&mut self) {
    if let Some(handle) = self.abort.take() {
      handle.abort();
    }
    self.callbacks.borrow_mut().clear();
  }
}

fn rpc_err(err: jsonrpsee::core::ClientError) -> JsValue {
  let message = err.to_string();
  let code = match &err {
    jsonrpsee::core::ClientError::Call(obj) => Some(obj.code()),
    _ => None,
  };

  let js_err = js_sys::Error::new(&message);
  if let Some(code) = code {
    let _ = js_sys::Reflect::set(&js_err, &"code".into(), &JsValue::from(code));
  }
  js_err.into()
}
