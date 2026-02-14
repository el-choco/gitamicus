use std::collections::HashMap;

// Represents a single drawable row in the commit graph.
#[derive(Clone, Debug)]
pub struct GraphNode {
    pub sha: String,
    pub paths: Vec<String>, // SVG path 'd' attributes
    pub path_colors: Vec<usize>, // Color index for each path
    pub cx: f64,            // Circle x
    pub cy: f64,            // Circle y
    pub r: f64,             // Circle radius
    pub color_index: usize, // To pick a color from a palette
}

const LANE_WIDTH: f64 = 20.0;
const ROW_HEIGHT: f64 = 32.0;
const CIRCLE_RADIUS: f64 = 6.5;

// A simplified graph layout algorithm. It's not perfect but a good start.
pub fn generate_graph(commits: &[(String, String, String, String, Vec<String>)]) -> Vec<GraphNode> {
    if commits.is_empty() {
        return vec![];
    }

    let mut nodes = Vec::new();
    let mut sha_to_row: HashMap<String, usize> = HashMap::new();
    for (i, (sha, _, _, _, _)) in commits.iter().enumerate() {
        sha_to_row.insert(sha.clone(), i);
    }

    let mut lane_heads: Vec<Option<String>> = Vec::new();
    let mut commit_lanes: HashMap<String, usize> = HashMap::new();

    // Assign lanes to commits
    for (_i, (sha, _, _, _, parents)) in commits.iter().enumerate() {
        let mut assigned_lane = None;

        // Try to reuse the lane of the first parent
        if let Some(parent_sha) = parents.get(0) {
            if let Some(&parent_lane) = commit_lanes.get(parent_sha) {
                if lane_heads.get(parent_lane).and_then(|h| h.as_ref()) == Some(parent_sha) {
                    assigned_lane = Some(parent_lane);
                }
            }
        }

        let lane = if let Some(l) = assigned_lane {
            l
        } else {
            // Find a new, free lane
            if let Some(free_lane) = lane_heads.iter().position(|l| l.is_none()) {
                free_lane
            } else {
                lane_heads.push(None);
                lane_heads.len() - 1
            }
        };

        commit_lanes.insert(sha.clone(), lane);
        if lane >= lane_heads.len() {
            lane_heads.resize(lane + 1, None);
        }
        lane_heads[lane] = Some(sha.clone());

        // For merge commits, free up the lanes of the other parents
        for (_idx, parent_sha) in parents.iter().enumerate().filter(|(i, _)| *i > 0) {
            if let Some(&p_lane) = commit_lanes.get(parent_sha) {
                if lane_heads.get(p_lane).and_then(|h| h.as_ref()) == Some(parent_sha) {
                    lane_heads[p_lane] = None;
                }
            }
        }
    }

    // Generate drawable nodes with SVG paths
    for (i, (sha, _, _, _, parents)) in commits.iter().enumerate() {
        let &current_lane = commit_lanes.get(sha).unwrap_or(&0);
        let mut paths = Vec::new();
        let mut path_colors = Vec::new();

        for parent_sha in parents {
            if let Some(&parent_row) = sha_to_row.get(parent_sha) {
                let &parent_lane = commit_lanes.get(parent_sha).unwrap_or(&0);

                let x1 = (current_lane as f64 * LANE_WIDTH) + LANE_WIDTH / 2.0;
                let y1 = (i as f64 * ROW_HEIGHT) + ROW_HEIGHT / 2.0;
                let x2 = (parent_lane as f64 * LANE_WIDTH) + LANE_WIDTH / 2.0;
                let y2 = (parent_row as f64 * ROW_HEIGHT) + ROW_HEIGHT / 2.0;

                let path = if current_lane == parent_lane {
                    // Straight line for same lane
                    format!("M {} {} L {} {}", x1, y1, x2, y2)
                } else {
                    // Quadratic Bezier curve for lane changes - smoother appearance
                    let control_y = (y1 + y2) / 2.0;
                    let control_x = if x1 < x2 { 
                        x1 + (x2 - x1) * 0.5 
                    } else { 
                        x2 + (x1 - x2) * 0.5 
                    };
                    format!("M {} {} Q {} {}, {} {}", x1, y1, control_x, control_y, x2, y2)
                };
                paths.push(path);
                // Use parent lane color for the path for better branch visualization
                path_colors.push(parent_lane % 8);
            }
        }

        nodes.push(GraphNode { 
            sha: sha.clone(), 
            paths, 
            path_colors,
            cx: (current_lane as f64 * LANE_WIDTH) + LANE_WIDTH / 2.0, 
            cy: ROW_HEIGHT / 2.0, 
            r: CIRCLE_RADIUS, 
            color_index: current_lane % 8 
        });
    }

    nodes
}