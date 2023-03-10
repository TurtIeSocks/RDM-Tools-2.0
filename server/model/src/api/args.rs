use super::*;

use geojson::JsonValue;

use crate::{
    api::{collection::Default, text::TextHelpers},
    utils::{get_enum, get_enum_by_geometry_string},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundsArg {
    pub min_lat: Precision,
    pub min_lon: Precision,
    pub max_lat: Precision,
    pub max_lon: Precision,
    pub last_seen: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReturnTypeArg {
    AltText,
    Text,
    SingleArray,
    MultiArray,
    SingleStruct,
    MultiStruct,
    Geometry,
    GeometryVec,
    Feature,
    FeatureVec,
    FeatureCollection,
    PoracleSingle,
    Poracle,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SortBy {
    GeoHash,
    ClusterCount,
    Random,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SpawnpointTth {
    All,
    Known,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DataPointsArg {
    Array(single_vec::SingleVec),
    Struct(single_struct::SingleStruct),
    Feature(Feature),
    FeatureCollection(FeatureCollection),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiQueryArgs {
    pub internal: Option<bool>,
    pub id: Option<bool>,
    pub name: Option<bool>,
    pub mode: Option<bool>,
    pub geofence_id: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum UnknownId {
    String(String),
    Number(u32),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Args {
    pub area: Option<GeoFormats>,
    pub benchmark_mode: Option<bool>,
    pub data_points: Option<DataPointsArg>,
    pub devices: Option<usize>,
    pub fast: Option<bool>,
    pub generations: Option<usize>,
    pub instance: Option<String>,
    pub min_points: Option<usize>,
    pub radius: Option<Precision>,
    pub return_type: Option<String>,
    pub routing_time: Option<i64>,
    pub only_unique: Option<bool>,
    pub last_seen: Option<u32>,
    pub save_to_db: Option<bool>,
    pub save_to_scanner: Option<bool>,
    pub route_chunk_size: Option<usize>,
    pub simplify: Option<bool>,
    pub geometry_type: Option<String>,
    pub sort_by: Option<SortBy>,
    pub tth: Option<SpawnpointTth>,
    pub mode: Option<String>,
    pub route_split_level: Option<usize>,
}

pub struct ArgsUnwrapped {
    pub area: FeatureCollection,
    pub benchmark_mode: bool,
    pub data_points: single_vec::SingleVec,
    pub devices: usize,
    pub fast: bool,
    pub generations: usize,
    pub instance: String,
    pub min_points: usize,
    pub radius: Precision,
    pub return_type: ReturnTypeArg,
    pub only_unique: bool,
    pub last_seen: u32,
    pub save_to_db: bool,
    pub save_to_scanner: bool,
    pub simplify: bool,
    pub sort_by: SortBy,
    pub tth: SpawnpointTth,
    pub mode: Type,
    pub route_split_level: usize,
}

impl Args {
    pub fn init(self, input: Option<&str>) -> ArgsUnwrapped {
        let Args {
            area,
            benchmark_mode,
            data_points,
            devices,
            fast,
            generations,
            instance,
            min_points,
            radius,
            return_type,
            routing_time,
            only_unique,
            last_seen,
            save_to_db,
            save_to_scanner,
            route_chunk_size,
            simplify,
            geometry_type,
            sort_by,
            tth,
            mode,
            route_split_level,
        } = self;
        let enum_type = get_enum_by_geometry_string(geometry_type);
        let (area, default_return_type) = if let Some(area) = area {
            (
                area.clone().to_collection(instance.clone(), enum_type),
                match area {
                    GeoFormats::Text(area) => {
                        if area.text_test() {
                            ReturnTypeArg::AltText
                        } else {
                            ReturnTypeArg::Text
                        }
                    }
                    GeoFormats::SingleArray(_) | GeoFormats::Bound(_) => ReturnTypeArg::SingleArray,
                    GeoFormats::MultiArray(_) => ReturnTypeArg::MultiArray,
                    GeoFormats::SingleStruct(_) => ReturnTypeArg::SingleStruct,
                    GeoFormats::MultiStruct(_) => ReturnTypeArg::MultiStruct,
                    GeoFormats::Geometry(_) => ReturnTypeArg::Geometry,
                    GeoFormats::GeometryVec(_) => ReturnTypeArg::GeometryVec,
                    GeoFormats::Feature(_) => ReturnTypeArg::Feature,
                    GeoFormats::FeatureVec(_) => ReturnTypeArg::FeatureVec,
                    GeoFormats::FeatureCollection(_) => ReturnTypeArg::FeatureCollection,
                    GeoFormats::Poracle(_) | GeoFormats::PoracleSingle(_) => ReturnTypeArg::Poracle,
                },
            )
        } else {
            (FeatureCollection::default(), ReturnTypeArg::SingleArray)
        };
        let benchmark_mode = benchmark_mode.unwrap_or(false);
        let data_points = if let Some(data_points) = data_points {
            match data_points {
                DataPointsArg::Struct(data_points) => data_points.to_single_vec(),
                DataPointsArg::Array(data_points) => data_points,
                DataPointsArg::Feature(data_points) => data_points.to_single_vec(),
                DataPointsArg::FeatureCollection(data_points) => data_points.to_single_vec(),
            }
        } else {
            vec![]
        };
        let devices = devices.unwrap_or(1);
        let fast = fast.unwrap_or(true);
        let generations = generations.unwrap_or(1);
        let instance = instance.unwrap_or("".to_string());
        let min_points = min_points.unwrap_or(1);
        let radius = radius.unwrap_or(70.0);
        let return_type = if let Some(return_type) = return_type {
            get_return_type(return_type, &default_return_type)
        } else {
            default_return_type
        };
        let only_unique = only_unique.unwrap_or(false);
        let last_seen = last_seen.unwrap_or(0);
        let save_to_db = save_to_db.unwrap_or(false);
        let save_to_scanner = save_to_scanner.unwrap_or(false);
        let simplify = simplify.unwrap_or(false);
        let sort_by = sort_by.unwrap_or(SortBy::GeoHash);
        let tth = tth.unwrap_or(SpawnpointTth::All);
        let mode = get_enum(mode);
        let route_split_level = if let Some(route_split_level) = route_split_level {
            if route_split_level.lt(&13) && route_split_level.gt(&0) {
                route_split_level
            } else {
                log::warn!(
                    "route_split_level only supports 1-12, {} was provided",
                    route_split_level
                );
                1
            }
        } else {
            1
        };
        if route_chunk_size.is_some() {
            log::warn!("route_chunk_size is now deprecated, please use route_split_level")
        }
        if routing_time.is_some() {
            log::warn!("routing_time is now deprecated, please use route_split_level")
        }
        if let Some(input) = input {
            log::info!(
                "[{}]: Instance: {} | Custom Area: {} | Custom Data Points: {}\nRadius: | {} Min Points: {} | Generations: {} | Routing Split Level: {} | Devices: {} | Fast: {}\nOnly Unique: {}, Last Seen: {}\nReturn Type: {:?}",
                input.to_uppercase(), instance, !area.features.is_empty(), !data_points.is_empty(), radius, min_points, generations, route_split_level, devices, fast, only_unique, last_seen, return_type,
            );
        };
        ArgsUnwrapped {
            area,
            benchmark_mode,
            data_points,
            devices,
            fast,
            generations,
            instance,
            min_points,
            radius,
            return_type,
            only_unique,
            last_seen,
            save_to_db,
            save_to_scanner,
            simplify,
            sort_by,
            tth,
            mode,
            route_split_level,
        }
    }
}

pub fn get_return_type(return_type: String, default_return_type: &ReturnTypeArg) -> ReturnTypeArg {
    match return_type.to_lowercase().replace("-", "_").as_str() {
        "alttext" | "alt_text" => ReturnTypeArg::AltText,
        "text" => ReturnTypeArg::Text,
        "array" => match *default_return_type {
            ReturnTypeArg::SingleArray => ReturnTypeArg::SingleArray,
            ReturnTypeArg::MultiArray => ReturnTypeArg::MultiArray,
            _ => ReturnTypeArg::SingleArray,
        },
        "singlearray" | "single_array" => ReturnTypeArg::SingleArray,
        "multiarray" | "multi_array" => ReturnTypeArg::MultiArray,
        "struct" => match *default_return_type {
            ReturnTypeArg::SingleStruct => ReturnTypeArg::SingleStruct,
            ReturnTypeArg::MultiStruct => ReturnTypeArg::MultiStruct,
            _ => ReturnTypeArg::SingleStruct,
        },
        "geometry" => ReturnTypeArg::Geometry,
        "geometryvec" | "geometry_vec" | "geometries" => ReturnTypeArg::GeometryVec,
        "singlestruct" | "single_struct" => ReturnTypeArg::SingleStruct,
        "multistruct" | "multi_struct" => ReturnTypeArg::MultiStruct,
        "feature" => ReturnTypeArg::Feature,
        "featurevec" | "feature_vec" => ReturnTypeArg::FeatureVec,
        "poracle" => ReturnTypeArg::Poracle,
        "featurecollection" | "feature_collection" => ReturnTypeArg::FeatureCollection,
        _ => default_return_type.clone(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub start_lat: Precision,
    pub start_lon: Precision,
    pub tile_server: String,
    pub scanner_type: String,
    pub logged_in: bool,
    pub dangerous: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stats {
    pub best_clusters: single_vec::SingleVec,
    pub best_cluster_point_count: usize,
    pub cluster_time: Precision,
    pub total_points: usize,
    pub points_covered: usize,
    pub total_clusters: usize,
    pub total_distance: Precision,
    pub longest_distance: Precision,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            best_clusters: vec![],
            best_cluster_point_count: 0,
            cluster_time: 0.,
            total_points: 0,
            points_covered: 0,
            total_clusters: 0,
            total_distance: 0.,
            longest_distance: 0.,
        }
    }
    pub fn log(&self, area: Option<String>) {
        let width = "=======================================================================";
        let get_row = |text: String, replace: bool| {
            format!(
                "  {}{}{}\n",
                text,
                width[..(width.len() - text.len())].replace("=", if replace { " " } else { "=" }),
                if replace { "||" } else { "==" }
            )
        };
        log::info!(
            "\n{}{}{}{}{}{}  {}==\n",
            get_row("[STATS] ".to_string(), false),
            if let Some(area) = area {
                if area.is_empty() {
                    "".to_string()
                } else {
                    get_row(format!("|| [AREA]: {}", area), true)
                }
            } else {
                "".to_string()
            },
            get_row(
                format!(
                    "|| [POINTS] Total: {} | Covered: {}",
                    self.total_points, self.points_covered,
                ),
                true
            ),
            get_row(
                format!(
                    "|| [CLUSTERS] Time: {}s | Total: {} | Avg Points: {}",
                    self.cluster_time as f32,
                    self.total_clusters,
                    if self.total_clusters > 0 {
                        self.total_points / self.total_clusters
                    } else {
                        0
                    },
                ),
                true
            ),
            get_row(
                format!(
                    "|| [BEST_CLUSTER] Amount: {:?} | Point Count: {}",
                    self.best_clusters.len(),
                    self.best_cluster_point_count,
                ),
                true
            ),
            get_row(
                format!(
                    "|| [DISTANCE] Total {} | Longest {} | Avg: {}",
                    self.total_distance as f32,
                    self.longest_distance as f32,
                    if self.total_clusters > 0 {
                        (self.total_distance / self.total_clusters as f64) as f32
                    } else {
                        0.
                    },
                ),
                true
            ),
            width,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    pub message: String,
    pub status: String,
    pub status_code: u16,
    pub data: Option<JsonValue>,
    pub stats: Option<Stats>,
}

impl Response {
    pub fn send_error(message: &str) -> Response {
        Response {
            message: message.to_string(),
            status: "error".to_string(),
            status_code: 500,
            data: None,
            stats: None,
        }
    }
}
