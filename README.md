# Kōji

<!-- [![GitHub Release](https://img.shields.io/github/release/TurtIeSocks/Koji.svg)](https://github.com/TurtIeSocks/Koji/releases/)
[![GitHub Contributors](https://img.shields.io/github/contributors/TurtIeSocks/Koji.svg)](https://github.com/TurtIeSocks/Koji/graphs/contributors/) -->

[![Discord](https://img.shields.io/discord/907337201044582452.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/EYYsKPVawn)

## Features

- Geofence manager, editor, and distribution tool
- API based clustering for automatic route making
- Fast route solver for quest scanning
- Geofence conversions into various formats

## Compatibility

- [RealDeviceMap (RDM)](https://github.com/realdevicemap/realdevicemap)

## Installation

### Standard

1. Clone this repo:

```bash
git clone https://github.com/TurtIeSocks/Koji.git
```

2. [Install NodeJS](https://nodejs.dev/en/learn/how-to-install-nodejs/)
3. [Install Rust](https://www.rust-lang.org/tools/install)
4. Copy the env file:

```bash
cd server && cp .env.example .env
```

5. Edit the env file: `nano .env`
   - Set the `DATABASE_URL` to your RDM database url
   - Temporarily set `NODE_ENV` to `development`
   - Set `PORT` to whatever you want
   - Set `START_LAT` and `START_LON` to wherever you want the map to start
6. Compile the client:

```bash
cd ../client && yarn install && yarn build
```

7. Compile the server:

```bash
cd ../server && cargo run -r
# you might have to also install pkg-config (`apt install pkg-config`)
```

### Docker (Recommended)

1. Get the docker-compose.yml example file:

```bash
curl https://raw.githubusercontent.com/TurtIeSocks/Koji/main/docker-compose.example.yml > docker-compose.yml
```

2. `nano docker-compose.yml`
3. Set the same env variables as above
4. `docker-compose pull`
5. `docker-compose up -d`

### Development

1. After installing Rust and Node
2. Install Cargo Watch:

```bash
cargo install cargo-watch
```

3. Install the VS Code Plugin [Rust Analyzer](https://marketplace.visualstudio.com/items?itemName=matklad.rust-analyzer)
4. Open one terminal:

```bash
cd server
# to compile in debug mode (faster recompiling but slower performance)
cargo watch -x run
# to compile in release mode (slower recompiling but faster performance)
cargo watch -x 'run -r'
```

5. In another terminal:

```bash
cd client && yarn install && yarn dev
```

6. A browser will automatically open to `localhost:{PORT}`

## API

General Types:

```rust
// Data Structs and Type Aliases
pub type PointArray<T = f64> = [T; 2];
pub type SingleVec<T = f64> = Vec<PointArray<T>>;
pub type MultiVec<T = f64> = Vec<Vec<PointArray<T>>>;

pub struct PointStruct<T: Float = f64> {
    pub lat: T,
    pub lon: T,
}
pub type SingleStruct<T = f64> = Vec<PointStruct<T>>;
pub type MultiStruct<T = f64> = Vec<Vec<PointStruct<T>>>;

// Accepted Area Inputs and Outputs:
  pub enum GeoFormats {
      Text(String),
      // can be either:
        // lat,lon\nlat,lon
        // or lat lon,lat lon
      SingleArray(SingleVec),
      MultiArray(MultiVec),
      SingleStruct(SingleStruct),
      MultiStruct(MultiStruct),
      Feature(Feature),
      FeatureVec(Vec<Feature>),
      FeatureCollection(FeatureCollection),
  }

// Return Types:
  pub enum ReturnType {
    AltText, // lat lon,lat lon
    Text, // lat,lon\nlat,lon
    SingleArray,
    MultiArray,
    SingleStruct,
    MultiStruct,
    Feature,
    FeatureVec,
    FeatureCollection,
}

// Data Input Types:
  pub enum DataPointsArg {
      Array(SingleVec),
      Struct(SingleStruct),
  }

// all API Fields
  pub struct Args {
      // The instance or area to lookup in the db to get geofence/data points
      // defaults to ""
      pub instance: Option<String>,

      // radius of the circle to use in calculations
      // defaults to 70m
      pub radius: Option<f64>,

      // min number of points to use with clustering
      // defaults to 1
      pub min_points: Option<usize>,

      // number of times to run through the clustering optimizations
      // defaults to 1
      pub generations: Option<usize>,

      // number of seconds (s) to run the routing algorithm (longer = better routes)
      // defaults to 1
      pub routing_time: Option<i64>,

      // number of devices - not implemented atm
      // defaults to 1
      pub devices: Option<usize>,

      // Custom list of data points to use in calculations - overrides all else
      // defaults to []
      pub data_points: Option<DataPointsArg>,

      // Custom area to use in the SQL query to get data points
      // defaults to empty FeatureCollection
      pub area: Option<GeoFormats>,

      // Run the fast algorithm or not
      // defaults to true
      pub fast: Option<bool>,

      // Format of how the data should be returned
      // defaults to AreaInput type or SingleArray if AreaInput is None
      pub return_type: Option<String>,

      // Only return stats
      // defaults to false
      pub benchmark_mode: Option<bool>,
  }

// Benchmark/Stats Struct
  pub struct Stats {
      pub best_cluster: Option<PointArray>,
      pub best_cluster_count: Option<u8>,
      pub cluster_time: Option<f64>,
      pub points_covered: Option<u32>,
      pub total_clusters: Option<u32>,
      pub total_distance: Option<u32>,
      pub longest_distance: Option<u32>,
  }

// Response Struct (what you will receive!)
  pub struct Response {
      pub message: String,
      pub status: String,
      pub status_code: u16,
      pub data: GeoFormats,
      pub stats: Stats,
  }
```

### /api/v1/calc/bootstrap

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area` OR `instance`
  - **Optional**:
    - `radius`
    - `return_type`

### /api/v1/calc/cluster

### /api/v1/calc/route

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area` OR `instance`
  - **Optional**:
    - `radius`
    - `return_type`
    - `min_points`
    - `generations`
    - `devices`
    - `data_points`
    - `fast`

### /api/v1/convert/data

- **Method:** `POST`
- **JSON Body**:
  - **Required**:
    - `area`
  - **Optional**:
    - `return_type`
