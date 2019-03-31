use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Deserializer};
use failure::Error;


pub const SCHEME: &str = env!("WOZ_WEB_SCHEME");
pub const NETLOC: &str = env!("WOZ_WEB_NETLOC");
pub const IDENTITY_POOL_ID: &str = env!("WOZ_IDENTITY_POOL_ID");
pub const IDENTITY_POOL_URL: &str = env!("WOZ_IDENTITY_POOL_URL");
pub const CLIENT_ID: &str = env!("WOZ_CLIENT_ID");
pub const S3_BUCKET_NAME: &str = env!("WOZ_S3_BUCKET_NAME");
pub const ENCRYPTION_PASSWORD: &str = env!("WOZ_ENCRYPTION_PASSWORD");
pub const ENCRYPTION_SALT: &str = env!("WOZ_ENCRYPTION_SALT");

pub static DEFAULT_PROJECT_LIB_RS: &str = include_str!(concat!(env!("WOZ_PROJECT_ROOT"), "/examples/seed-app/src/lib.rs"));

// Default icons are included in the bin. This will make it bigger so
// maybe in the future these should be downloaded to the user's local
// filesystem on install.
//
// Making these all static even though they will only be accessed via
// DEFAULT_ICONS so that it's a compile error if the default icon
// files don't exist.
static DEFAULT_ICON_48X48: &'static [u8; 6623] = include_bytes!(concat!(env!("WOZ_CLI_PROJECT_ROOT"), "/resources/icons/48x48.png"));
static DEFAULT_ICON_72X72: &'static [u8; 6623] = include_bytes!(concat!(env!("WOZ_CLI_PROJECT_ROOT"), "/resources/icons/72x72.png"));
static DEFAULT_ICON_96X96: &'static [u8; 6623] = include_bytes!(concat!(env!("WOZ_CLI_PROJECT_ROOT"), "/resources/icons/96x96.png"));
static DEFAULT_ICON_144X144: &'static [u8; 6623] = include_bytes!(concat!(env!("WOZ_CLI_PROJECT_ROOT"), "/resources/icons/144x144.png"));
static DEFAULT_ICON_168X168: &'static [u8; 6623] = include_bytes!(concat!(env!("WOZ_CLI_PROJECT_ROOT"), "/resources/icons/168x168.png"));
static DEFAULT_ICON_192X192: &'static [u8; 6623] = include_bytes!(concat!(env!("WOZ_CLI_PROJECT_ROOT"), "/resources/icons/192x192.png"));

// iOS icon sizes
static DEFAULT_ICON_152X152: &'static [u8; 6623] = include_bytes!(concat!(env!("WOZ_CLI_PROJECT_ROOT"), "/resources/icons/152x152.png"));
static DEFAULT_ICON_167X167: &'static [u8; 6623] = include_bytes!(concat!(env!("WOZ_CLI_PROJECT_ROOT"), "/resources/icons/167x167.png"));
static DEFAULT_ICON_180X180: &'static [u8; 6623] = include_bytes!(concat!(env!("WOZ_CLI_PROJECT_ROOT"), "/resources/icons/180x180.png"));

lazy_static!{
    pub static ref DEFAULT_ICONS: HashMap<&'static str, Vec<u8>> = {
        let mut m = HashMap::new();
        m.insert("48x48", DEFAULT_ICON_48X48.to_vec());
        m.insert("72x72", DEFAULT_ICON_72X72.to_vec());
        m.insert("96x96", DEFAULT_ICON_96X96.to_vec());
        m.insert("144x144", DEFAULT_ICON_144X144.to_vec());
        m.insert("152x152", DEFAULT_ICON_152X152.to_vec());
        m.insert("167x167", DEFAULT_ICON_167X167.to_vec());
        m.insert("168x168", DEFAULT_ICON_168X168.to_vec());
        m.insert("180x180", DEFAULT_ICON_180X180.to_vec());
        m.insert("192x192", DEFAULT_ICON_192X192.to_vec());
        m
    };
}

static DEFAULT_SPLASH_IPHONE5: &'static [u8; 37880] = include_bytes!("../resources/splashscreens/iphone5_splash.png");
static DEFAULT_SPLASH_IPHONE6: &'static [u8; 39604] = include_bytes!("../resources/splashscreens/iphone6_splash.png");
static DEFAULT_SPLASH_IPHONEPLUS: &'static [u8; 51706] = include_bytes!("../resources/splashscreens/iphoneplus_splash.png");
static DEFAULT_SPLASH_IPHONEX: &'static [u8; 44797] = include_bytes!("../resources/splashscreens/iphonex_splash.png");
static DEFAULT_SPLASH_IPHONEXR: &'static [u8; 42280] = include_bytes!("../resources/splashscreens/iphonexr_splash.png");
static DEFAULT_SPLASH_IPHONEXSMAX: &'static [u8; 54950] = include_bytes!("../resources/splashscreens/iphonexsmax_splash.png");
static DEFAULT_SPLASH_IPAD: &'static [u8; 54919] = include_bytes!("../resources/splashscreens/ipad_splash.png");
static DEFAULT_SPLASH_IPADPRO1: &'static [u8; 60819] = include_bytes!("../resources/splashscreens/ipadpro1_splash.png");
static DEFAULT_SPLASH_IPADPRO3: &'static [u8; 62394] = include_bytes!("../resources/splashscreens/ipadpro3_splash.png");
static DEFAULT_SPLASH_IPADPRO2: &'static [u8; 71374] = include_bytes!("../resources/splashscreens/ipadpro2_splash.png");

lazy_static!{
    pub static ref DEFAULT_SPLASHSCREENS: HashMap<&'static str, Vec<u8>> = {
        let mut m = HashMap::new();
        m.insert("iphone5", DEFAULT_SPLASH_IPHONE5.to_vec());
        m.insert("iphone6", DEFAULT_SPLASH_IPHONE6.to_vec());
        m.insert("iphoneplus", DEFAULT_SPLASH_IPHONEPLUS.to_vec());
        m.insert("iphonex", DEFAULT_SPLASH_IPHONEX.to_vec());
        m.insert("iphonexr", DEFAULT_SPLASH_IPHONEXR.to_vec());
        m.insert("iphonexsmax", DEFAULT_SPLASH_IPHONEXSMAX.to_vec());
        m.insert("ipad", DEFAULT_SPLASH_IPAD.to_vec());
        m.insert("ipadpro1", DEFAULT_SPLASH_IPADPRO1.to_vec());
        m.insert("ipadpro3", DEFAULT_SPLASH_IPADPRO3.to_vec());
        m.insert("ipadpro2", DEFAULT_SPLASH_IPADPRO2.to_vec());
        m
    };
}

#[derive(Debug, Serialize)]
pub enum Lib {
    WasmBindgen,
    StdWeb,
    Unknown(String)
}

impl<'de> Deserialize<'de> for Lib {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "wasm-bindgen" => Lib::WasmBindgen,
            "std-web" => Lib::StdWeb,
            _ => Lib::Unknown(s),
        })
    }
}

#[derive(Debug, Serialize)]
pub enum Environment {
    Release,
    Development,
    Unknown(String)
}

impl<'de> Deserialize<'de> for Environment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "release" => Environment::Release,
            "development" => Environment::Development,
            _ => Environment::Unknown(s),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ProjectId(pub String);

impl<'de> Deserialize<'de> for ProjectId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        if s.chars().all(char::is_alphanumeric) {
            Ok(ProjectId(s))
        } else {
            Err(serde::de::Error::custom(String::from("must be alphanumeric")))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Icons {
    pub path_48x48: PathBuf,
    pub path_72x72: PathBuf,
    pub path_96x96: PathBuf,
    pub path_144x144: PathBuf,
    pub path_168x168: PathBuf,
    pub path_192x192: PathBuf,
}

impl Icons {
    pub fn to_vec(&self) -> Vec<(&'static str, &PathBuf)>{
        vec![
            ("48x48", &self.path_48x48),
            ("72x72", &self.path_72x72),
            ("96x96", &self.path_96x96),
            ("144x144", &self.path_144x144),
            ("168x168", &self.path_168x168),
            ("192x192", &self.path_192x192),
        ]
    }
}

#[derive(Debug, Deserialize)]
pub struct SplashScreens {
    iphone5: PathBuf,
    iphone6: PathBuf,
    iphoneplus: PathBuf,
    iphonex: PathBuf,
    iphonexr: PathBuf,
    iphonexsmax: PathBuf,
    ipad: PathBuf,
    ipadpro1: PathBuf,
    ipadpro3: PathBuf,
    ipadpro2: PathBuf,
}

impl SplashScreens {
    pub fn to_vec(&self) -> Vec<(&'static str, &PathBuf)>{
        vec![
            ("iphone5", &self.iphone5),
            ("iphone6", &self.iphone6),
            ("iphoneplus", &self.iphoneplus),
            ("iphonex", &self.iphonex),
            ("iphonexr", &self.iphonexr),
            ("iphonexsmax", &self.iphonexsmax),
            ("ipad", &self.ipad),
            ("ipadpro1", &self.ipadpro1),
            ("ipadpro3", &self.ipadpro3),
            ("ipadpro2", &self.ipadpro2),
        ]
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub project_id: ProjectId,
    pub lib: Option<Lib>,
    pub name: String,
    pub short_name: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub env: Option<Environment>,
    pub wasm_path: PathBuf,
    pub icons: Option<Icons>,
    pub splashscreens: Option<SplashScreens>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project_id: ProjectId(String::from("default")),
            lib: Some(Lib::WasmBindgen),
            name: String::from("My App"),
            short_name: Some(String::from("App")),
            author: None,
            description: Some(String::from("App built with woz.sh")),
            env: Some(Environment::Release),
            wasm_path: PathBuf::new(),
            icons: None,
            splashscreens: None,
        }
    }
}

pub fn default_home_path() -> Result<PathBuf, Error> {
    let home: String = std::env::var_os("XDG_CONFIG_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .map(|v| v.into_string().expect("Unable to parse $HOME to string"))
        .expect("No home");
    let mut buf = PathBuf::new();
    buf.push(home);
    buf.push(".woz");
    Ok(buf)
}

#[test]
// TODO only compile on macOS
fn default_home_path_test() {
    let user = std::env::var_os("USER")
        .map(|v| v.into_string().expect("Could not parse $USER to string"))
        .expect("Could not get a $USER");
    let path_str = format!("/Users/{}/.woz", user);
    assert_eq!(PathBuf::from(path_str), default_home_path().unwrap());
}
