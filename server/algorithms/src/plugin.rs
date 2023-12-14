use std::fmt::Display;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::s2::create_cell_map;
use crate::utils;
use model::api::single_vec::SingleVec;
use rayon::iter::{Either, IntoParallelIterator, ParallelIterator};

#[derive(Debug)]
pub enum Folder {
    Routing,
    Clustering,
    Bootstrap,
}

impl Display for Folder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Folder::Routing => write!(f, "routing"),
            Folder::Clustering => write!(f, "clustering"),
            Folder::Bootstrap => write!(f, "bootstrap"),
        }
    }
}

#[derive(Debug)]
pub struct Plugin {
    plugin_path: String,
    interpreter: String,
    args: Vec<String>,
    pub plugin: String,
    pub split_level: u64,
}

pub type JoinFunction = fn(&Plugin, Vec<SingleVec>) -> SingleVec;

trait ParseCoord {
    fn parse_next_coord(&mut self) -> Option<f64>;
}

impl ParseCoord for std::str::Split<'_, &str> {
    fn parse_next_coord(&mut self) -> Option<f64> {
        if let Some(coord) = self.next() {
            if let Ok(coord) = coord.parse::<f64>() {
                return Some(coord);
            }
        }
        None
    }
}

impl Plugin {
    pub fn new(
        plugin: &str,
        folder: Folder,
        route_split_level: u64,
        routing_args: &str,
    ) -> std::io::Result<Self> {
        let mut plugin_path = format!("algorithms/src/{folder}/plugins/{plugin}");
        let mut interpreter = match plugin.split(".").last() {
            Some("py") => "python3",
            Some("js") => "node",
            Some("sh") => "bash",
            Some("ts") => "ts-node",
            val => {
                if plugin == val.unwrap_or("") {
                    &plugin_path
                } else {
                    ""
                }
            }
        }
        .to_string();
        let args = routing_args
            .split_whitespace()
            .skip_while(|arg| !arg.starts_with("--"))
            .map(|arg| arg.to_string())
            .collect::<Vec<String>>();

        for (index, pre_arg) in routing_args
            .split_whitespace()
            .take_while(|arg| !arg.starts_with("--"))
            .enumerate()
        {
            log::info!("[PLUGIN PARSER] {index} | pre_arg: {}", pre_arg);
            if index == 0 {
                interpreter = pre_arg.to_string();
            } else if index == 1 {
                plugin_path = format!("algorithms/src/{folder}/plugins/{pre_arg}");
            } else {
                log::warn!("Unrecognized argument: {pre_arg} for plugin: {plugin}")
            }
        }

        if interpreter.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Unrecognized plugin, please create a PR to add support for it",
            ));
        };
        let path = Path::new(&plugin_path);
        if path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{plugin} is a directory, not a file, something may not be right with the provided args"),
            ));
        } else if path.exists() {
            plugin_path = path.display().to_string();
            log::info!("{interpreter} {plugin_path} {}", args.join(" "));
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "{plugin} does not exist{}",
                    if plugin == "tsp" {
                        ", rerun the OR Tools Script"
                    } else {
                        ""
                    }
                ),
            ));
        }

        Ok(Plugin {
            plugin: plugin.to_string(),
            plugin_path,
            interpreter,
            split_level: route_split_level,
            args,
        })
    }

    pub fn run_multi<T>(
        &self,
        points: &SingleVec,
        joiner: Option<T>,
    ) -> Result<SingleVec, std::io::Error>
    where
        T: Fn(&Self, Vec<SingleVec>) -> SingleVec,
    {
        let handlers = if self.split_level == 0 {
            vec![self.run(utils::stringify_points(&points))?]
        } else {
            create_cell_map(&points, self.split_level)
                .into_values()
                .collect::<Vec<SingleVec>>()
                .into_par_iter()
                .filter_map(|x| self.run(utils::stringify_points(&x)).ok())
                .collect()
        };
        if let Some(joiner) = joiner {
            Ok(joiner(self, handlers))
        } else {
            Ok(handlers.into_iter().flatten().collect())
        }
    }

    pub fn run(&self, input: String) -> Result<SingleVec, std::io::Error> {
        log::info!("spawning {} child process", self.plugin);

        let time = Instant::now();

        let mut child = Command::new(&self.interpreter);
        if self.plugin_path != self.interpreter {
            child.arg(&self.plugin_path);
        };
        let mut child = match child
            .args(self.args.iter())
            .args(&["--input", &input])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) => return Err(err),
        };

        let mut stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "failed to open stdin",
                ));
            }
        };

        match stdin.write_all(input.as_bytes()) {
            Ok(_) => match stdin.flush() {
                Ok(_) => {}
                Err(err) => {
                    log::error!("failed to flush stdin: {}", err);
                }
            },
            Err(err) => {
                log::error!("failed to write to stdin: {}", err)
            }
        };

        let output = match child.wait_with_output() {
            Ok(result) => result,
            Err(err) => return Err(err),
        };
        let output = String::from_utf8_lossy(&output.stdout);
        // let mut output_indexes = output
        //     .split(",")
        //     .filter_map(|s| s.trim().parse::<usize>().ok())
        //     .collect::<Vec<usize>>();
        let (invalid, mut output_result): (Vec<&str>, SingleVec) = output
            .split_ascii_whitespace()
            .into_iter()
            .collect::<Vec<_>>()
            .into_par_iter()
            .partition_map(|s| {
                let mut iter: std::str::Split<'_, &str> = s.trim().split(",");
                let lat = iter.parse_next_coord();
                let lng = iter.parse_next_coord();
                if lat.is_none() || lng.is_none() {
                    Either::Left(s)
                } else {
                    Either::Right([lat.unwrap(), lng.unwrap()])
                }
            });
        if let Some(first) = output_result.first() {
            if let Some(last) = output_result.last() {
                if first == last {
                    output_result.pop();
                }
            }
        }
        if !invalid.is_empty() {
            log::warn!(
                "Some invalid results were returned from the plugin: `{}`",
                invalid.join(", ")
            );
        }
        if output_result.is_empty() {
            Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "no valid output from child process \n{}\noutput should return points in the following format: `lat,lng lat,lng`",
                        output
                    ),
                ))
        } else {
            log::info!(
                "{} child process finished in {}s",
                self.plugin,
                time.elapsed().as_secs_f32()
            );
            // Ok(output_indexes.into_iter().map(|i| points[i]).collect())
            Ok(output_result)
        }
    }
}
