use hashbrown::HashSet;
use model::api::{cluster_mode::ClusterMode, single_vec::SingleVec, GetBbox, Precision};

use rayon::{
    prelude::{IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use rstar::RTree;
use std::{io::Write, time::Instant};

use crate::{
    clustering::rtree::{cluster::Cluster, point::Point},
    rtree::{self, point::ToPoint},
    s2,
    utils::info_log,
};

pub struct Greedy {
    cluster_mode: ClusterMode,
    cluster_split_level: u64,
    max_clusters: usize,
    min_points: usize,
    radius: f64,
}

impl Default for Greedy {
    fn default() -> Self {
        Greedy {
            cluster_mode: ClusterMode::Balanced,
            cluster_split_level: 1,
            max_clusters: usize::MAX,
            min_points: 1,
            radius: 70.,
        }
    }
}

impl<'a> Greedy {
    pub fn set_cluster_mode(&mut self, cluster_mode: ClusterMode) -> &mut Self {
        self.cluster_mode = cluster_mode;
        self
    }
    pub fn set_radius(&mut self, radius: f64) -> &mut Self {
        self.radius = radius;
        self
    }
    pub fn set_max_clusters(&mut self, max_clusters: usize) -> &mut Self {
        self.max_clusters = max_clusters;
        self
    }
    pub fn set_min_points(&mut self, min_points: usize) -> &mut Self {
        self.min_points = min_points;
        self
    }
    pub fn set_cluster_split_level(&mut self, cluster_split_level: u64) -> &mut Self {
        self.cluster_split_level = cluster_split_level;
        self
    }

    pub fn run(&'a self, points: &SingleVec) -> SingleVec {
        let time = Instant::now();
        log::info!("starting algorithm with {} data points", points.len());

        let return_set = if self.cluster_split_level == 1 {
            self.setup(points)
        } else {
            let cell_maps = s2::create_cell_map(&points, self.cluster_split_level);

            let mut return_set = HashSet::new();
            std::thread::scope(|s| {
                let mut handlers = vec![];
                for (key, values) in cell_maps.iter() {
                    log::debug!("Cell: {} | Points: {}", key, values.len());
                    let thread = s.spawn(move || self.setup(values));
                    handlers.push(thread);
                }
                let handlers: Vec<std::thread::ScopedJoinHandle<'_, HashSet<Point>>> = cell_maps
                    .iter()
                    .map(|(key, values)| {
                        log::debug!("Cell: {} | Points: {}", key, values.len());
                        s.spawn(move || self.setup(values))
                    })
                    .collect();
                log::info!("created {} threads", handlers.len());
                for thread in handlers {
                    match thread.join() {
                        Ok(results) => {
                            return_set.extend(results);
                        }
                        Err(e) => {
                            log::error!("error joining thread: {:?}", e)
                        }
                    }
                }
            });

            return_set
        };

        log::info!("finished in {:.2}s", time.elapsed().as_secs_f32());
        return_set.into_iter().map(|p| p.center).collect()
    }

    fn generate_clusters(&self, point: &Point, neighbors: Vec<&Point>) -> HashSet<Point> {
        let mut set = HashSet::<Point>::new();
        for neighbor in neighbors.iter() {
            for i in 0..=7 {
                let ratio = i as Precision / 8 as Precision;
                let new_point = point.interpolate(neighbor, ratio, 0., 0.);
                set.insert(new_point);
                if self.cluster_mode == ClusterMode::Balanced {
                    for wiggle in vec![0.00025, 0.0001] {
                        let wiggle_lat: f64 = wiggle / 2.;
                        let wiggle_lon = wiggle;
                        let random_point =
                            point.interpolate(neighbor, ratio, wiggle_lat, wiggle_lon);
                        set.insert(random_point);
                        let random_point =
                            point.interpolate(neighbor, ratio, wiggle_lat, -wiggle_lon);
                        set.insert(random_point);
                        let random_point =
                            point.interpolate(neighbor, ratio, -wiggle_lat, wiggle_lon);
                        set.insert(random_point);
                        let random_point =
                            point.interpolate(neighbor, ratio, -wiggle_lat, -wiggle_lon);
                        set.insert(random_point);
                    }
                }
            }
        }
        set.insert(point.to_owned());
        set
    }

    fn gen_estimated_clusters(&self, tree: &RTree<Point>) -> Vec<Point> {
        let tree_points: Vec<&Point> = tree.iter().map(|p| p).collect();

        let clusters = tree_points
            .par_iter()
            .map(|point| {
                let neighbors = tree
                    .locate_all_at_point(&point.center)
                    .into_iter()
                    .collect();
                self.generate_clusters(point, neighbors)
            })
            .reduce(HashSet::new, |a, b| a.union(&b).cloned().collect());

        clusters.into_iter().collect::<Vec<Point>>()
    }

    fn get_s2_clusters(&self, points: &SingleVec) -> Vec<Point> {
        let bbox = points.get_bbox().unwrap();
        s2::get_region_cells(bbox[1], bbox[3], bbox[0], bbox[2], 22)
            .0
            .par_iter()
            .map(|cell| cell.to_point(self.radius))
            .collect()
    }

    fn setup(&'a self, points: &SingleVec) -> HashSet<Point> {
        let time = Instant::now();
        let point_tree: RTree<Point> = rtree::spawn(self.radius, points);
        log::info!("created point tree in {:.2}s", time.elapsed().as_secs_f32());

        let time = Instant::now();
        let clusters: Vec<Point> = match self.cluster_mode {
            ClusterMode::Better | ClusterMode::Best => self.get_s2_clusters(points),
            ClusterMode::Fast => self.gen_estimated_clusters(&point_tree),
            _ => {
                let time = Instant::now();
                let neighbor_tree: RTree<Point> = rtree::spawn(self.radius * 2., points);
                log::info!("created neighbor tree {:.2}s", time.elapsed().as_secs_f32());
                self.gen_estimated_clusters(&neighbor_tree)
            }
        };
        log::info!(
            "created possible {} clusters in {:.2}s",
            clusters.len(),
            time.elapsed().as_secs_f32(),
        );

        let time = Instant::now();
        let mut clusters_with_data: Vec<Cluster> = clusters
            .par_iter()
            .filter_map(|cluster| {
                let mut points: Vec<&Point> = point_tree
                    .locate_all_at_point(&cluster.center)
                    .collect::<Vec<&Point>>();
                if point_tree.contains(cluster) && points.is_empty() {
                    points.push(cluster)
                }
                if points.is_empty() {
                    log::debug!("Empty");
                    None
                } else {
                    Some(Cluster::new(
                        cluster,
                        points.into_iter(),
                        vec![].into_iter(),
                    ))
                }
            })
            .collect();
        log::info!(
            "associated points with {} clusters in {:.2}s",
            clusters_with_data.len(),
            time.elapsed().as_secs_f32(),
        );

        let time = Instant::now();
        clusters_with_data.par_sort_by(|a, b| b.all.len().cmp(&a.all.len()));
        log::info!(
            "sorted clusters by points covered in {:.2}s",
            time.elapsed().as_secs_f32(),
        );

        let solution = self.cluster(clusters_with_data);

        let solution = self.dedupe(solution);
        solution
    }

    fn cluster(&'a self, clusters_with_data: Vec<Cluster<'a>>) -> HashSet<Cluster<'a>> {
        let time = Instant::now();
        log::info!("starting initial solution",);

        let mut new_clusters = HashSet::<Cluster>::new();
        let mut blocked_points = HashSet::<&Point>::new();

        let mut highest = 100;
        let mut total_iterations = 0;
        let mut current_iteration = 0;
        let mut stdout = std::io::stdout();

        'greedy: while highest > self.min_points && new_clusters.len() < self.max_clusters {
            let local_clusters = clusters_with_data
                .par_iter()
                .filter_map(|cluster| {
                    if new_clusters.contains(cluster) {
                        None
                    } else {
                        let points: HashSet<&Point> = cluster
                            .all
                            .iter()
                            .filter_map(|p| {
                                if blocked_points.contains(p) {
                                    None
                                } else {
                                    Some(*p)
                                }
                            })
                            .collect();
                        if points.len() < self.min_points {
                            None
                        } else {
                            Some(Cluster {
                                point: cluster.point,
                                points,
                                all: cluster.all.iter().map(|p| *p).collect(),
                            })
                        }
                    }
                })
                .collect::<Vec<Cluster>>();

            let mut best = 0;
            'cluster: for cluster in local_clusters.into_iter() {
                if new_clusters.len() >= self.max_clusters {
                    break 'greedy;
                }
                let length = cluster.points.len() + 1;
                if length > best {
                    best = length;
                }
                if length >= highest {
                    if new_clusters.contains(&cluster) || length == 0 {
                        continue;
                    }
                    // let mut count = 0;
                    for point in cluster.points.iter() {
                        if blocked_points.contains(point) {
                            continue 'cluster;
                            // count += 1;
                            // if count >= min_points {
                            //     break;
                            // }
                        }
                    }
                    // if count >= min_points {
                    for point in cluster.points.iter() {
                        blocked_points.insert(point);
                    }
                    new_clusters.insert(cluster);
                    // }
                }
            }
            if best + 1 < highest && best > 0 {
                total_iterations = best * 2 - (self.min_points * 2) + current_iteration;
            }
            current_iteration += 1;
            highest = best;

            if highest >= self.min_points {
                stdout
                    .write(
                        info_log(
                            "algorithms::clustering::greedy",
                            format!(
                                "Progress: {:.2}% | Clusters: {}",
                                (current_iteration as f32 / total_iterations as f32) * 100.,
                                new_clusters.len()
                            ),
                        )
                        .as_bytes(),
                    )
                    .unwrap();
                stdout.flush().unwrap();
            } else {
                stdout.write(format!("\n",).as_bytes()).unwrap();
            }
        }

        log::info!(
            "finished initial solution in {:.2}s",
            time.elapsed().as_secs_f32()
        );
        log::info!("Initial solution size: {}", new_clusters.len());
        new_clusters
    }

    fn dedupe(&self, initial: HashSet<Cluster>) -> HashSet<Point> {
        let time = Instant::now();
        log::info!("starting deduping");

        // _debug_clusters(&initial, "dedupe");

        let mut seen_points: HashSet<&Point> = HashSet::new();
        let mut solution: HashSet<Point> = initial
            .iter()
            .filter_map(|cluster| {
                let unique_points = cluster
                    .all
                    .iter()
                    .collect::<Vec<&&Point>>()
                    .par_iter()
                    .filter(|p| {
                        initial
                            .iter()
                            .find(|c| c.point != cluster.point && c.all.contains(**p))
                            .is_none()
                    })
                    .count();

                if unique_points == 0 || unique_points < self.min_points {
                    None
                } else {
                    seen_points.extend(cluster.all.iter());
                    Some(*cluster.point)
                }
            })
            .collect();

        if self.min_points == 1 {
            // log::warn!("Extra needed, current size: {}", solution.len());

            // let mut count = 0;
            for cluster in initial {
                let valid = cluster
                    .all
                    .iter()
                    .find(|p| !seen_points.contains(*p))
                    .is_some();
                if valid {
                    solution.insert(*cluster.point);
                    seen_points.extend(cluster.all.iter());
                    // count += 1;
                }
            }
            // log::warn!("Extra clusters: {}", count);
        }

        log::info!("finished deduping in {:.2}s", time.elapsed().as_secs_f32());
        log::info!("Deduped solution size: {}", solution.len());
        solution
    }
}
