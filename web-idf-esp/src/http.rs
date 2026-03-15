use std::sync::{Arc, Mutex};

use esp_idf_svc::http::{Method, server::{EspHttpConnection, EspHttpServer}};
use log::info;


pub struct HttpServerManager<'a>{
    server: EspHttpServer<'a>,
    configured: Arc<Mutex<bool>>,
}

impl HttpServerManager<'_> {
    pub fn new() -> anyhow::Result<Self> {
        let server = EspHttpServer::new(&Default::default())?;
        Ok(Self {
            server,
            configured: Arc::new(Mutex::new(false)),
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

    pub fn init_ap_pages(&mut self) -> anyhow::Result<()> {
        *self.configured.lock().unwrap() = false;

        
        let configured_clone1 = self.configured.clone();
        self.fn_handler("/generate_204", Method::Get, move |req| {

            let ok = *configured_clone1.lock().unwrap();

            // info!("Received request for /hotspot-detect.html from {}", req.connection().remote_addr());

            info!("Received {:?} request for {} configured={}", req.method(), req.uri(), ok);
            
            
            if ok { 
                let mut resp = req.into_ok_response()?;        
                resp.write(b"<HTML><BODY>Success</BODY></HTML>")?;
            } else {
                let mut resp = req.into_response(302, None, &[("Location", "/")])?;
                resp.write(b"<HTML><BODY>Not configured</BODY></HTML>")?;
            }
            Ok(())
        })?;

        let configured_clone1 = self.configured.clone();
        self.fn_handler("/hotspot-detect.html", Method::Get, move |req| {

            let ok = *configured_clone1.lock().unwrap();

            // info!("Received request for /hotspot-detect.html from {}", req.connection().remote_addr());

            info!("Received {:?} request for {} configured={} V2", req.method(), req.uri(), ok);
            
            if ok {  
                let mut resp = req.into_ok_response()?;       
                resp.write(b"<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 3.2//EN\">
<HTML>
<HEAD>
	<TITLE>Success</TITLE>
</HEAD>
<BODY>
	Success
</BODY>
</HTML>")?;
            } else {let mut resp = req.into_response(302, None, &[("Location", "/")])?;
                resp.write(b"<HTML><BODY>Not configured</BODY></HTML>")?;
            }
            Ok(())
        })?;

        let configured_clone1 = self.configured.clone();
        self.fn_handler("/connecttest.txt", Method::Get, move |req| {

            let ok = *configured_clone1.lock().unwrap();

            // info!("Received request for /hotspot-detect.html from {}", req.connection().remote_addr());

            info!("Received {:?} request for {} configured={}", req.method(), req.uri(), ok);
            
            if ok {  
                let mut resp = req.into_ok_response()?;       
                resp.write(b"Microsoft Connect Test")?;
            } else {
                let mut resp = req.into_response(302, None, &[("Location", "/")])?;
                resp.write(b"Not configured")?;
            }
            Ok(())
        })?;

        self.fn_handler("/", Method::Get, |req| {

            // info!("Received request for / from {}", req.connection().remote_addr());

            info!("Received {:?} request for {}", req.method(), req.uri());

            let html = r#"
                <html>
                <body>
                <h1>ESP32 Setup</h1>
                <form method="POST" action="/connect">
                WiFi SSID:<input name="ssid"><br>
                WiFi PASS:<input name="pass" type="password"><br>
                <button>Save</button>
                </form>
                </body>
                </html>
                "#;

            let mut resp = req.into_ok_response()?;
            resp.write(html.as_bytes())?;
            Ok(())
        })?;

        let configured_clone2 = self.configured.clone();
        self.fn_handler("/connect", Method::Post, move |mut req| {

            // info!("Received request for /connect from {}", req.connection().remote_addr());

            info!("Received {:?} request for {}", req.method(), req.uri());
            

            let mut buf = [0;512];
            let len = req.read(&mut buf)?;

            let body = core::str::from_utf8(&buf[..len]).unwrap();

            let ssid = parse(body,"ssid");
            let pass = parse(body,"pass");

            info!("Received WiFi credentials: ssid={}, pass={}", ssid, pass);
            // tx_clone.send(WifiCommand::Connect{ssid,pass}).ok();


            *configured_clone2.lock().unwrap() = true;

            let mut resp = req.into_ok_response()?;
            resp.write(b"Saved!")?;
            Ok(())
        })?;

        fn parse(body:&str,key:&str)->String{
            body.split('&')
                .find(|p|p.starts_with(key))
                .and_then(|v|v.split('=').nth(1))
                .unwrap_or("")
                .to_string()
        }

        Ok(())
    }
}

