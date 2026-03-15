use esp_idf_svc::http::{Method, server::{EspHttpConnection, EspHttpServer}};


pub struct HttpServerManager<'a>{
    server: EspHttpServer<'a>,
}

impl HttpServerManager<'_> {
    pub fn new() -> anyhow::Result<Self> {
        let server = EspHttpServer::new(&Default::default())?;
        Ok(Self {
            server
        })
    }

    pub fn fn_handler<F>(
        &mut self,
        uri: &str,
        method: Method,
        f: F,
    ) -> anyhow::Result<()>
    where
        F: for<'r> Fn(esp_idf_svc::http::server::Request<&mut EspHttpConnection>) -> anyhow::Result<()> + Send + 'static,
    {
        self.server.fn_handler(uri, method, f)?;
        Ok(())
    }
}

