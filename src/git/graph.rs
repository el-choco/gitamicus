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
const BEZIER_CONTROL_OFFSET: f64 = 0.4; // Control point offset for smoother curves

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
    let mut branch_colors: HashMap<String, usize> = HashMap::new(); // Track branch colors
    let mut next_color: usize = 0;

    // First pass: assign lanes
    for (_i, (sha, _, _, _, parents)) in commits.iter().enumerate() {
        let mut assigned_lane = None;

        // Try to reuse the lane of the first parent for continuity
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

    // Second pass: assign colors (process in reverse to handle parent-to-child inheritance)
    for (sha, _, _, _, parents) in commits.iter().rev() {
        if !branch_colors.contains_key(sha) {
            // Check if first parent has a color - if so, inherit it
            if let Some(parent_sha) = parents.get(0) {
                if let Some(&parent_color) = branch_colors.get(parent_sha) {
                    branch_colors.insert(sha.clone(), parent_color);
                } else {
                    // Parent not colored yet, assign new color
                    branch_colors.insert(sha.clone(), next_color % 8);
                    next_color += 1;
                }
            } else {
                // No parent, assign new color
                branch_colors.insert(sha.clone(), next_color % 8);
                next_color += 1;
            }
        }
    }

    // Generate drawable nodes with SVG paths using smooth Bezier curves
    for (i, (sha, _, _, _, parents)) in commits.iter().enumerate() {
        let &current_lane = commit_lanes.get(sha).unwrap_or(&0);
        let current_color = branch_colors.get(sha).copied().unwrap_or(current_lane % 8);
        let mut paths = Vec::new();
        let mut path_colors = Vec::new();

        for parent_sha in parents {
            if let Some(&parent_row) = sha_to_row.get(parent_sha) {
                let &parent_lane = commit_lanes.get(parent_sha).unwrap_or(&0);
                let parent_color = branch_colors.get(parent_sha).copied().unwrap_or(parent_lane % 8);

                let x1 = (current_lane as f64 * LANE_WIDTH) + LANE_WIDTH / 2.0;
                let y1 = (i as f64 * ROW_HEIGHT) + ROW_HEIGHT / 2.0;
                let x2 = (parent_lane as f64 * LANE_WIDTH) + LANE_WIDTH / 2.0;
                let y2 = (parent_row as f64 * ROW_HEIGHT) + ROW_HEIGHT / 2.0;

                // Use smooth quadratic Bezier curves for ALL connections
                let path = if current_lane == parent_lane {
                    // Even for same lane, use a subtle curve for smoother appearance
                    let control_y = (y1 + y2) / 2.0;
                    format!("M {} {} Q {} {}, {} {}", x1, y1, x1, control_y, x2, y2)
                } else {
                    // Quadratic Bezier curve for lane changes - smoother appearance
                    let dy = y2 - y1;
                    let control_y = y1 + dy * BEZIER_CONTROL_OFFSET;
                    let control_x = if x1 < x2 { 
                        x1 + (x2 - x1) * BEZIER_CONTROL_OFFSET 
                    } else { 
                        x1 - (x1 - x2) * BEZIER_CONTROL_OFFSET 
                    };
                    format!("M {} {} Q {} {}, {} {}", x1, y1, control_x, control_y, x2, y2)
                };
                paths.push(path);
                // Use the color of the commit that owns this connection
                path_colors.push(parent_color);
            }
        }

        nodes.push(GraphNode { 
            sha: sha.clone(), 
            paths, 
            path_colors,
            cx: (current_lane as f64 * LANE_WIDTH) + LANE_WIDTH / 2.0, 
            cy: ROW_HEIGHT / 2.0, 
            r: CIRCLE_RADIUS, 
            color_index: current_color
        });
    }

    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_commits() {
        let commits = vec![];
        let nodes = generate_graph(&commits);
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_single_commit() {
        let commits = vec![
            ("abc123".to_string(), "Initial commit".to_string(), "Author".to_string(), "2024-01-01".to_string(), vec![])
        ];
        let nodes = generate_graph(&commits);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].sha, "abc123");
        assert_eq!(nodes[0].paths.len(), 0); // No parents, no paths
    }

    #[test]
    fn test_linear_history() {
        let commits = vec![
            ("commit3".to_string(), "Third".to_string(), "Author".to_string(), "2024-01-03".to_string(), vec!["commit2".to_string()]),
            ("commit2".to_string(), "Second".to_string(), "Author".to_string(), "2024-01-02".to_string(), vec!["commit1".to_string()]),
            ("commit1".to_string(), "First".to_string(), "Author".to_string(), "2024-01-01".to_string(), vec![]),
        ];
        let nodes = generate_graph(&commits);
        assert_eq!(nodes.len(), 3);
        // All commits should be in the same lane
        assert_eq!(nodes[0].color_index, nodes[1].color_index);
        assert_eq!(nodes[1].color_index, nodes[2].color_index);
    }

    #[test]
    fn test_branch_split() {
        let commits = vec![
            ("commit3".to_string(), "Branch commit".to_string(), "Author".to_string(), "2024-01-03".to_string(), vec!["commit1".to_string()]),
            ("commit2".to_string(), "Main commit".to_string(), "Author".to_string(), "2024-01-02".to_string(), vec!["commit1".to_string()]),
            ("commit1".to_string(), "Initial".to_string(), "Author".to_string(), "2024-01-01".to_string(), vec![]),
        ];
        let nodes = generate_graph(&commits);
        assert_eq!(nodes.len(), 3);
        // Branches should have different colors
        assert_ne!(nodes[0].color_index, nodes[1].color_index);
    }

    #[test]
    fn test_merge_commit() {
        let commits = vec![
            ("merge".to_string(), "Merge".to_string(), "Author".to_string(), "2024-01-04".to_string(), vec!["commit2".to_string(), "commit3".to_string()]),
            ("commit3".to_string(), "Branch".to_string(), "Author".to_string(), "2024-01-03".to_string(), vec!["commit1".to_string()]),
            ("commit2".to_string(), "Main".to_string(), "Author".to_string(), "2024-01-02".to_string(), vec!["commit1".to_string()]),
            ("commit1".to_string(), "Initial".to_string(), "Author".to_string(), "2024-01-01".to_string(), vec![]),
        ];
        let nodes = generate_graph(&commits);
        assert_eq!(nodes.len(), 4);
        // Merge commit should have two paths
        assert_eq!(nodes[0].paths.len(), 2);
    }
}