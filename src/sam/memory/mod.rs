pub mod config;
pub mod human;
pub mod location;
pub mod thing;
pub mod observation;
// pub mod service;
// pub mod file_storage;
// pub mod notification;
pub mod room;
// pub mod setting;
// pub mod storage_location;
// pub mod web_sessions;
// pub mod cached_wikipedia_summary;
// pub mod human_face_encoding;
pub mod cache;
pub mod storage;

// Re-export types for convenience
pub use config::Config;
pub use human::Human;
pub use location::*;
pub use thing::*;
pub use observation::*;
// pub use service::*;
// pub use file_storage::*;
// pub use notification::*;
pub use room::*;
// pub use setting::*;
// pub use storage_location::*;
// pub use web_sessions::*;
// pub use cached_wikipedia_summary::*;
// pub use human_face_encoding::*;

// ===== Shared error_chain! block =====
use error_chain::error_chain;
error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
        TokioPg(tokio_postgres::Error);
        Hound(hound::Error);
        PostError(rouille::input::post::PostError);
        ParseFloatError(std::num::ParseFloatError);
        SerdeJsonError(serde_json::Error);
        // TchError(tch::TchError);
    }
}

// ===== Shared utility types and enums =====

use serde::{Serialize, Deserialize};
use std::fmt;
use std::env;

// PostgresServer config struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostgresServer {
	pub db_name: String,
    pub username: String,
    pub password: String,
	pub address: String
}
impl Default for PostgresServer {
    fn default() -> Self {
        Self::new()
    }
}
impl PostgresServer {
    pub fn new() -> PostgresServer {
        let db_name = env::var("PG_DBNAME").expect("$PG_DBNAME is not set");
        let username = env::var("PG_USER").expect("$PG_USER is not set");
        let password = env::var("PG_PASS").expect("$PG_PASS is not set");
        let address = env::var("PG_ADDRESS").expect("$PG_ADDRESS is not set");
        PostgresServer{
            db_name, 
            username, 
            password, 
            address
        }
    }
}

// Not tracked in SQL
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PostgresQueries {
    pub queries: Vec<PGCol>, 
    pub query_columns: Vec<String>,
    pub append: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PGCol {
    String(String),
    Number(i32),
    Boolean(bool),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeepVisionResult {
    pub id: String,
    pub whoio: Option<WhoioResult>,
    pub probability: f64,
    pub left: i64,
    pub bottom: i64,
    pub right: i64,
    pub top: i64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WhoioResult {
    pub id: String,
    pub directory: String,
    pub left: i64,
    pub bottom: i64,
    pub right: i64,
    pub top: i64
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ObservationType {
    UNKNOWN,
    SEEN,
    HEARD
}
impl fmt::Display for ObservationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::str::FromStr for ObservationType {
    type Err = ();
    fn from_str(input: &str) -> std::result::Result<ObservationType, Self::Err> {
        match input {
            "UNKNOWN"  => Ok(ObservationType::UNKNOWN),
            "SEEN"  => Ok(ObservationType::SEEN),
            "HEARD"  => Ok(ObservationType::HEARD),
            _      => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ObservationObjects {
    #[allow(non_camel_case_types)]
    QR_CODE,
    #[allow(non_camel_case_types)]
    PERSON,
    #[allow(non_camel_case_types)]
    BICYCLE,
    #[allow(non_camel_case_types)]
    CAR,
    #[allow(non_camel_case_types)]
    MOTORBIKE,
    #[allow(non_camel_case_types)]
    AEROPLANE,
    #[allow(non_camel_case_types)]
    BUS,
    #[allow(non_camel_case_types)]
    TRAIN,
    #[allow(non_camel_case_types)]
    TRUCK,
    #[allow(non_camel_case_types)]
    BOAT,
    #[allow(non_camel_case_types)]
    TRAFFIC_LIGHT,
    #[allow(non_camel_case_types)]
    FIRE_HYDRANT,
    #[allow(non_camel_case_types)]
    STOP_SIGN,
    #[allow(non_camel_case_types)]
    PARKING_METER,
    #[allow(non_camel_case_types)]
    BENCH,
    #[allow(non_camel_case_types)]
    BIRD,
    #[allow(non_camel_case_types)]
    CAT,
    #[allow(non_camel_case_types)]
    DOG,
    #[allow(non_camel_case_types)]
    HORSE,
    #[allow(non_camel_case_types)]
    SHEEP,
    #[allow(non_camel_case_types)]
    COW,
    #[allow(non_camel_case_types)]
    ELEPHANT,
    #[allow(non_camel_case_types)]
    BEAR,
    #[allow(non_camel_case_types)]
    ZEBRA,
    #[allow(non_camel_case_types)]
    GIRAFFE,
    #[allow(non_camel_case_types)]
    BACKPACK,
    #[allow(non_camel_case_types)]
    UMBRELLA,
    #[allow(non_camel_case_types)]
    HANDBAG,
    #[allow(non_camel_case_types)]
    TIE,
    #[allow(non_camel_case_types)]
    SUITCASE,
    #[allow(non_camel_case_types)]
    FRISBEE,
    #[allow(non_camel_case_types)]
    SKIS,
    #[allow(non_camel_case_types)]
    SNOWBOARD,
    #[allow(non_camel_case_types)]
    SPORTS_BALL,
    #[allow(non_camel_case_types)]
    KITE,
    #[allow(non_camel_case_types)]
    BASEBALL_BAT,
    #[allow(non_camel_case_types)]
    SKATEBOARD,
    #[allow(non_camel_case_types)]
    SURFBOARD,
    #[allow(non_camel_case_types)]
    TENNIS_RACKET,
    #[allow(non_camel_case_types)]
    BOTTLE,
    #[allow(non_camel_case_types)]
    WINE_GLASS,
    #[allow(non_camel_case_types)]
    CUP,
    #[allow(non_camel_case_types)]
    FORK,
    #[allow(non_camel_case_types)]
    KNIFE,
    #[allow(non_camel_case_types)]
    SPOON,
    #[allow(non_camel_case_types)]
    BOWL,
    #[allow(non_camel_case_types)]
    BANANA,
    #[allow(non_camel_case_types)]
    APPLE,
    #[allow(non_camel_case_types)]
    SANDWICH,
    #[allow(non_camel_case_types)]
    ORANGE,
    #[allow(non_camel_case_types)]
    BROCCOLI,
    #[allow(non_camel_case_types)]
    CARROT,
    #[allow(non_camel_case_types)]
    HOT_DOG,
    #[allow(non_camel_case_types)]
    PIZZA,
    #[allow(non_camel_case_types)]
    DONUT,
    #[allow(non_camel_case_types)]
    CAKE,
    #[allow(non_camel_case_types)]
    CHAIR,
    #[allow(non_camel_case_types)]
    SOFA,
    #[allow(non_camel_case_types)]
    POTTED_PLANT,
    #[allow(non_camel_case_types)]
    BED,
    #[allow(non_camel_case_types)]
    DINING_TABLE,
    #[allow(non_camel_case_types)]
    TOILET,
    #[allow(non_camel_case_types)]
    TV_MONITOR,
    #[allow(non_camel_case_types)]
    LAPTOP,
    #[allow(non_camel_case_types)]
    MOUSE,
    #[allow(non_camel_case_types)]
    REMOTE,
    #[allow(non_camel_case_types)]
    KEYBOARD,
    #[allow(non_camel_case_types)]
    CELL_PHONE,
    #[allow(non_camel_case_types)]
    MICROWAVE,
    #[allow(non_camel_case_types)]
    OVEN,
    #[allow(non_camel_case_types)]
    TOASTER,
    #[allow(non_camel_case_types)]
    SINK,
    #[allow(non_camel_case_types)]
    REFRIGERATOR,
    #[allow(non_camel_case_types)]
    BOOK,
    #[allow(non_camel_case_types)]
    CLOCK,
    #[allow(non_camel_case_types)]
    VASE,
    #[allow(non_camel_case_types)]
    SCISSORS,
    #[allow(non_camel_case_types)]
    TEDDY_BEAR,
    #[allow(non_camel_case_types)]
    HAIR_DRIER,
    #[allow(non_camel_case_types)]
    TOOTHBRUSH
}
impl fmt::Display for ObservationObjects {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::str::FromStr for ObservationObjects {
    type Err = ();
    fn from_str(input: &str) -> std::result::Result<ObservationObjects, Self::Err> {
        match input {
            "QR_CODE"  => Ok(ObservationObjects::QR_CODE),
            "PERSON"  => Ok(ObservationObjects::PERSON),
            "BICYCLE"  => Ok(ObservationObjects::BICYCLE),
            "CAR"  => Ok(ObservationObjects::CAR),
            "MOTORBIKE"  => Ok(ObservationObjects::MOTORBIKE),
            "AEROPLANE"  => Ok(ObservationObjects::AEROPLANE),
            "BUS"  => Ok(ObservationObjects::BUS),
            "TRAIN"  => Ok(ObservationObjects::TRAIN),
            "TRUCK"  => Ok(ObservationObjects::TRUCK),
            "BOAT"  => Ok(ObservationObjects::BOAT),
            "TRAFFIC_LIGHT"  => Ok(ObservationObjects::TRAFFIC_LIGHT),
            "FIRE_HYDRANT"  => Ok(ObservationObjects::FIRE_HYDRANT),
            "STOP_SIGN"  => Ok(ObservationObjects::STOP_SIGN),
            "PARKING_METER"  => Ok(ObservationObjects::PARKING_METER),
            "BENCH"  => Ok(ObservationObjects::BENCH),
            "BIRD"  => Ok(ObservationObjects::BIRD),
            "CAT"  => Ok(ObservationObjects::CAT),
            "DOG"  => Ok(ObservationObjects::DOG),
            "HORSE"  => Ok(ObservationObjects::HORSE),
            "SHEEP"  => Ok(ObservationObjects::SHEEP),
            "COW"  => Ok(ObservationObjects::COW),
            "ELEPHANT"  => Ok(ObservationObjects::ELEPHANT),
            "BEAR"  => Ok(ObservationObjects::BEAR),
            "ZEBRA"  => Ok(ObservationObjects::ZEBRA),
            "GIRAFFE"  => Ok(ObservationObjects::GIRAFFE),
            "BACKPACK"  => Ok(ObservationObjects::BACKPACK),
            "UMBRELLA"  => Ok(ObservationObjects::UMBRELLA),
            "HANDBAG"  => Ok(ObservationObjects::HANDBAG),
            "TIE"  => Ok(ObservationObjects::TIE),
            "SUITCASE"  => Ok(ObservationObjects::SUITCASE),
            "FRISBEE"  => Ok(ObservationObjects::FRISBEE),
            "SKIS"  => Ok(ObservationObjects::SKIS),
            "SNOWBOARD"  => Ok(ObservationObjects::SNOWBOARD),
            "SPORTS_BALL"  => Ok(ObservationObjects::SPORTS_BALL),
            "KITE"  => Ok(ObservationObjects::KITE),
            "BASEBALL_BAT"  => Ok(ObservationObjects::BASEBALL_BAT),
            "SKATEBOARD"  => Ok(ObservationObjects::SKATEBOARD),
            "SURFBOARD"  => Ok(ObservationObjects::SURFBOARD),
            "TENNIS_RACKET"  => Ok(ObservationObjects::TENNIS_RACKET),
            "BOTTLE"  => Ok(ObservationObjects::BOTTLE),
            "WINE_GLASS"  => Ok(ObservationObjects::WINE_GLASS),
            "CUP"  => Ok(ObservationObjects::CUP),
            "FORK"  => Ok(ObservationObjects::FORK),
            "KNIFE"  => Ok(ObservationObjects::KNIFE),
            "SPOON"  => Ok(ObservationObjects::SPOON),
            "BOWL"  => Ok(ObservationObjects::BOWL),
            "BANANA"  => Ok(ObservationObjects::BANANA),
            "APPLE"  => Ok(ObservationObjects::APPLE),
            "SANDWICH"  => Ok(ObservationObjects::SANDWICH),
            "ORANGE"  => Ok(ObservationObjects::ORANGE),
            "BROCCOLI"  => Ok(ObservationObjects::BROCCOLI),
            "CARROT"  => Ok(ObservationObjects::CARROT),
            "HOT_DOG"  => Ok(ObservationObjects::HOT_DOG),
            "PIZZA"  => Ok(ObservationObjects::PIZZA),
            "DONUT"  => Ok(ObservationObjects::DONUT),
            "CAKE"  => Ok(ObservationObjects::CAKE),
            "CHAIR"  => Ok(ObservationObjects::CHAIR),
            "SOFA"  => Ok(ObservationObjects::SOFA),
            "POTTED_PLANT"  => Ok(ObservationObjects::POTTED_PLANT),
            "BED"  => Ok(ObservationObjects::BED),
            "DINING_TABLE"  => Ok(ObservationObjects::DINING_TABLE),
            "TOILET"  => Ok(ObservationObjects::TOILET),
            "TV_MONITOR"  => Ok(ObservationObjects::TV_MONITOR),
            "LAPTOP"  => Ok(ObservationObjects::LAPTOP),
            "MOUSE"  => Ok(ObservationObjects::MOUSE),
            "REMOTE"  => Ok(ObservationObjects::REMOTE),
            "KEYBOARD"  => Ok(ObservationObjects::KEYBOARD),
            "CELL_PHONE"  => Ok(ObservationObjects::CELL_PHONE),
            "MICROWAVE"  => Ok(ObservationObjects::MICROWAVE),
            "OVEN"  => Ok(ObservationObjects::OVEN),
            "SINK"  => Ok(ObservationObjects::SINK),
            "REFRIGERATOR"  => Ok(ObservationObjects::REFRIGERATOR),
            "BOOK"  => Ok(ObservationObjects::BOOK),
            "CLOCK"  => Ok(ObservationObjects::CLOCK),
            "VASE"  => Ok(ObservationObjects::VASE),
            "SCISSORS"  => Ok(ObservationObjects::SCISSORS),
            "TEDDY_BEAR"  => Ok(ObservationObjects::TEDDY_BEAR),
            "HAIR_DRIER"  => Ok(ObservationObjects::HAIR_DRIER),
            "TOOTHBRUSH"  => Ok(ObservationObjects::TOOTHBRUSH),
            _      => Err(()),
        }
    }
}
