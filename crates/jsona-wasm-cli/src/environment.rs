use anyhow::anyhow;
use futures::FutureExt;
use js_sys::{Function, Promise, Uint8Array};
use jsona_util::environment::Environment;
use std::{
    io,
    pin::Pin,
    task::{self, Poll},
};
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use url::Url;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{spawn_local, JsFuture};

pub(crate) struct JsAsyncRead {
    fut: Option<JsFuture>,
    f: Function,
}

impl JsAsyncRead {
    fn new(cb: Function) -> Self {
        Self { fut: None, f: cb }
    }
}

impl AsyncRead for JsAsyncRead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> task::Poll<std::io::Result<()>> {
        if self.fut.is_none() {
            let this = JsValue::null();
            let ret: JsValue = match self.f.call1(&this, &JsValue::from(buf.remaining())) {
                Ok(val) => val,
                Err(error) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("{:?}", error),
                    )));
                }
            };

            let promise = match Promise::try_from(ret) {
                Ok(p) => p,
                Err(err) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("{:?}", err),
                    )));
                }
            };

            self.fut = Some(JsFuture::from(promise));
        }

        if let Some(fut) = self.fut.as_mut() {
            match fut.poll_unpin(cx) {
                task::Poll::Ready(val) => {
                    let res = match val {
                        Ok(chunk) => {
                            let arr = js_sys::Uint8Array::from(chunk).to_vec();
                            if !arr.is_empty() {
                                buf.put_slice(&arr);
                            }

                            Ok(())
                        }
                        Err(err) => Err(io::Error::new(io::ErrorKind::Other, format!("{:?}", err))),
                    };

                    self.fut = None;

                    Poll::Ready(res)
                }
                task::Poll::Pending => Poll::Pending,
            }
        } else {
            unreachable!()
        }
    }
}

impl JsAsyncWrite {
    fn new(cb: Function) -> Self {
        Self { fut: None, f: cb }
    }
}

pub(crate) struct JsAsyncWrite {
    fut: Option<JsFuture>,
    f: Function,
}

impl AsyncWrite for JsAsyncWrite {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> task::Poll<Result<usize, std::io::Error>> {
        if self.fut.is_none() {
            let this = JsValue::null();

            let ret: JsValue = match self.f.call1(&this, &Uint8Array::from(buf).into()) {
                Ok(val) => val,
                Err(error) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("{:?}", error),
                    )));
                }
            };

            let promise = match Promise::try_from(ret) {
                Ok(p) => p,
                Err(err) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("{:?}", err),
                    )));
                }
            };

            self.fut = Some(JsFuture::from(promise));
        }

        if let Some(fut) = self.fut.as_mut() {
            match fut.poll_unpin(cx) {
                task::Poll::Ready(val) => {
                    let res = match val {
                        Ok(num_written) => {
                            let n = num_written.as_f64().unwrap_or(0.0).floor() as usize;
                            Ok(n)
                        }
                        Err(err) => Err(io::Error::new(io::ErrorKind::Other, format!("{:?}", err))),
                    };

                    self.fut = None;

                    Poll::Ready(res)
                }
                task::Poll::Pending => Poll::Pending,
            }
        } else {
            unreachable!()
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> task::Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> task::Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Clone)]
pub(crate) struct WasmEnvironment {
    js_now: Function,
    js_env_var: Function,
    js_atty_stderr: Function,
    js_on_stdin: Function,
    js_on_stdout: Function,
    js_on_stderr: Function,
    js_read_file: Function,
    js_write_file: Function,
    js_fetch_file: Function,
    js_root_uri: Function,
}

impl From<JsValue> for WasmEnvironment {
    fn from(val: JsValue) -> Self {
        Self {
            js_now: js_sys::Reflect::get(&val, &JsValue::from_str("js_now"))
                .unwrap()
                .into(),
            js_env_var: js_sys::Reflect::get(&val, &JsValue::from_str("js_env_var"))
                .unwrap()
                .into(),
            js_atty_stderr: js_sys::Reflect::get(&val, &JsValue::from_str("js_atty_stderr"))
                .unwrap()
                .into(),
            js_on_stdin: js_sys::Reflect::get(&val, &JsValue::from_str("js_on_stdin"))
                .unwrap()
                .into(),
            js_on_stdout: js_sys::Reflect::get(&val, &JsValue::from_str("js_on_stdout"))
                .unwrap()
                .into(),
            js_on_stderr: js_sys::Reflect::get(&val, &JsValue::from_str("js_on_stderr"))
                .unwrap()
                .into(),
            js_read_file: js_sys::Reflect::get(&val, &JsValue::from_str("js_read_file"))
                .unwrap()
                .into(),
            js_write_file: js_sys::Reflect::get(&val, &JsValue::from_str("js_write_file"))
                .unwrap()
                .into(),
            js_fetch_file: js_sys::Reflect::get(&val, &JsValue::from_str("js_fetch_file"))
                .unwrap()
                .into(),
            js_root_uri: js_sys::Reflect::get(&val, &JsValue::from_str("js_root_uri"))
                .unwrap()
                .into(),
        }
    }
}

// SAFETY: we're in a single-threaded WASM environment.
unsafe impl Send for WasmEnvironment {}
unsafe impl Sync for WasmEnvironment {}

#[async_trait::async_trait(?Send)]
impl Environment for WasmEnvironment {
    type Stdin = JsAsyncRead;
    type Stdout = JsAsyncWrite;
    type Stderr = JsAsyncWrite;

    fn now(&self) -> OffsetDateTime {
        let this = JsValue::null();
        let res: JsValue = self.js_now.call0(&this).unwrap();
        let s: String = js_sys::Date::from(res).to_iso_string().into();
        OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339).unwrap()
    }

    fn spawn<F>(&self, fut: F)
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send,
    {
        spawn_local(async move {
            fut.await;
        })
    }

    fn spawn_local<F>(&self, fut: F)
    where
        F: std::future::Future + 'static,
    {
        spawn_local(async move {
            fut.await;
        })
    }

    fn env_var(&self, name: &str) -> Option<String> {
        let this = JsValue::null();
        let res: JsValue = self
            .js_env_var
            .call1(&this, &JsValue::from_str(name))
            .unwrap();
        res.as_string()
    }

    fn atty_stderr(&self) -> bool {
        let this = JsValue::null();
        let res: JsValue = self.js_atty_stderr.call0(&this).unwrap();
        res.as_bool().unwrap_or(false)
    }

    fn stdin(&self) -> Self::Stdin {
        JsAsyncRead::new(self.js_on_stdin.clone())
    }

    fn stdout(&self) -> Self::Stdout {
        JsAsyncWrite::new(self.js_on_stdout.clone())
    }

    fn stderr(&self) -> Self::Stderr {
        JsAsyncWrite::new(self.js_on_stderr.clone())
    }

    async fn read_file(&self, path: &Url) -> Result<Vec<u8>, anyhow::Error> {
        let path_str = JsValue::from_str(path.as_str());
        let this = JsValue::null();
        let res: JsValue = self.js_read_file.call1(&this, &path_str).unwrap();

        let ret = JsFuture::from(Promise::from(res))
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(Uint8Array::from(ret).to_vec())
    }

    async fn write_file(&self, path: &Url, bytes: &[u8]) -> Result<(), anyhow::Error> {
        let path_str = JsValue::from_str(path.as_str());
        let this = JsValue::null();
        let res: JsValue = self
            .js_write_file
            .call2(&this, &path_str, &JsValue::from(Uint8Array::from(bytes)))
            .unwrap();
        let value = JsFuture::from(Promise::from(res))
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(serde_wasm_bindgen::from_value(value).map_err(|err| anyhow!("{err}"))?)
    }

    async fn fetch_file(&self, path: &Url) -> Result<Vec<u8>, anyhow::Error> {
        let path_str = JsValue::from_str(path.as_str());
        let this = JsValue::null();
        let res: JsValue = self.js_fetch_file.call1(&this, &path_str).unwrap();

        let ret = JsFuture::from(Promise::from(res))
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(Uint8Array::from(ret).to_vec())
    }

    fn root_uri(&self) -> Option<Url> {
        let this = JsValue::null();
        let res: JsValue = self.js_root_uri.call0(&this).unwrap();
        res.as_string().and_then(|v| v.parse().ok())
    }
}
