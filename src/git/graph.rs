use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct GraphNode {
    pub sha: String,
    pub paths: Vec<String>,      // SVG path 'd' attributes
    pub path_colors: Vec<usize>, // Color index for each path
    pub cx: f64,                 // Circle center x
    pub cy: f64,                 // Circle center y
    pub r: f64,                  // Circle radius
    pub color_index: usize,      // Main color for this node
}

pub const GRAPH_COLORS: &[&str] = &[
    "#4A90E2", // Blue
    "#F5A623", // Orange
    "#D0021B", // Red
    "#F8E71C", // Yellow
    "#7ED321", // Green
    "#9013FE", // Purple
    "#50E3C2", // Cyan
    "#F8A0D8", // Pink
];

const LANE_WIDTH: f64 = 30.0;
const ROW_HEIGHT: f64 = 40.0;
const CIRCLE_RADIUS: f64 = 5.5;
const LINE_OFFSET: f64 = LANE_WIDTH / 2.0;

pub fn generate_graph(commits: &[(String, String, String, String, Vec<String>)]) -> Vec<GraphNode> {
    if commits.is_empty() {
        return vec![];
    }

    let mut sha_to_row: HashMap<String, usize> = HashMap::new();
    for (i, (sha, _, _, _, _)) in commits.iter().enumerate() {
        sha_to_row.insert(sha.clone(), i);
    }

    let (commit_lanes, lane_colors) = assign_lanes_and_colors(commits);

    let mut nodes = Vec::new();

    for (current_row, (sha, _, _, _, parents)) in commits.iter().enumerate() {
        let current_lane = *commit_lanes.get(sha).unwrap_or(&0);
        let current_color_idx = *lane_colors.get(&current_lane).unwrap_or(&0);

        let (paths, path_colors) = generate_paths(
            current_row,
            current_lane,
            current_color_idx,
            parents,
            &sha_to_row,
            &commit_lanes,
            &lane_colors,
        );

        let cx = (current_lane as f64) * LANE_WIDTH + LINE_OFFSET;
        let cy = (current_row as f64) * ROW_HEIGHT + ROW_HEIGHT / 2.0;

        nodes.push(GraphNode {
            sha: sha.clone(),
            paths,
            path_colors,
            cx,
            cy,
            r: CIRCLE_RADIUS,
            color_index: current_color_idx,
        });
    }

    nodes
}

fn assign_lanes_and_colors(
    commits: &[(String, String, String, String, Vec<String>)],
) -> (HashMap<String, usize>, HashMap<usize, usize>) {
    let mut commit_lanes: HashMap<String, usize> = HashMap::new();
    let mut lane_colors: HashMap<usize, usize> = HashMap::new();
    let mut active_lanes: Vec<Option<String>> = Vec::new();
    let mut next_color: usize = 0;

    for (sha, _, _, _, parents) in commits.iter() {
        let assigned_lane = if let Some(parent_sha) = parents.first() {
            if let Some(&parent_lane) = commit_lanes.get(parent_sha) {
                if active_lanes
                    .get(parent_lane)
                    .and_then(|l| l.as_ref())
                    .map(|l| l == parent_sha)
                    .unwrap_or(false)
                {
                    Some(parent_lane)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let lane = assigned_lane.unwrap_or_else(|| {
            if let Some(free) = active_lanes.iter().position(|l| l.is_none()) {
                free
            } else {
                active_lanes.push(None);
                active_lanes.len() - 1
            }
        });

        while active_lanes.len() <= lane {
            active_lanes.push(None);
        }

        commit_lanes.insert(sha.clone(), lane);
        active_lanes[lane] = Some(sha.clone());

        if !lane_colors.contains_key(&lane) {
            lane_colors.insert(lane, next_color % GRAPH_COLORS.len());
            next_color += 1;
        }

        for parent_sha in parents.iter().skip(1) {
            if let Some(&parent_lane) = commit_lanes.get(parent_sha) {
                if active_lanes
                    .get(parent_lane)
                    .and_then(|l| l.as_ref())
                    .map(|l| l == parent_sha)
                    .unwrap_or(false)
                {
                    active_lanes[parent_lane] = None;
                }
            }
        }
    }

    (commit_lanes, lane_colors)
}

fn generate_paths(
    current_row: usize,
    current_lane: usize,
    current_color: usize,
    parents: &[String],
    sha_to_row: &HashMap<String, usize>,
    commit_lanes: &HashMap<String, usize>,
    lane_colors: &HashMap<usize, usize>,
) -> (Vec<String>, Vec<usize>) {
    let mut paths = Vec::new();
    let mut path_colors = Vec::new();

    let x1 = (current_lane as f64) * LANE_WIDTH + LINE_OFFSET;
    let y1 = (current_row as f64) * ROW_HEIGHT + ROW_HEIGHT / 2.0;

    for parent_sha in parents {
        if let Some(&parent_row) = sha_to_row.get(parent_sha) {
            let parent_lane = *commit_lanes.get(parent_sha).unwrap_or(&0);
            let parent_color = *lane_colors.get(&parent_lane).unwrap_or(&0);

            let x2 = (parent_lane as f64) * LANE_WIDTH + LINE_OFFSET;
            let y2 = (parent_row as f64) * ROW_HEIGHT + ROW_HEIGHT / 2.0;

            let path_d = if current_lane == parent_lane {
                // Same lane: straight vertical line
                format!("M {} {} L {} {}", x1, y1, x2, y2)
            } else {
                // Different lane: smooth cubic Bezier curve
                let mid_y = (y1 + y2) / 2.0;
                format!(
                    "M {} {} C {} {} {} {} {} {}",
                    x1, y1, x1, mid_y, x2, mid_y, x2, y2
                )
            };

            paths.push(path_d);
            path_colors.push(parent_color);
        }
    }

    (paths, path_colors)
}