/// Generate MQTT topics in a tree structure
///
/// For example with max_depth=2, topics_per_node=3, prefix="test", start_num=1:
/// - test00001
/// - test00001/01, test00001/02, test00001/03
/// - test00001/01/01, test00001/01/02, ..., test00001/03/03

pub struct TopicGenerator {
    prefix: String,
    base_topic_index: usize,
    topics_per_node: usize,
    max_depth: usize,
}

impl TopicGenerator {
    pub fn new(
        prefix: String,
        base_topic_index: usize,
        topics_per_node: usize,
        max_depth: usize,
    ) -> Self {
        TopicGenerator {
            prefix,
            base_topic_index,
            topics_per_node,
            max_depth,
        }
    }

    pub fn generate_all(&self) -> Vec<String> {
        let mut topics = Vec::new();
        let base_topic = format!("{}{:05}", self.prefix, self.base_topic_index);

        // Add root topic
        topics.push(base_topic.clone());

        // Generate topics for each depth level
        for depth in 1..=self.max_depth {
            self.generate_at_depth(&mut topics, &base_topic, depth, Vec::new());
        }

        topics
    }

    fn generate_at_depth(
        &self,
        topics: &mut Vec<String>,
        base: &str,
        target_depth: usize,
        path: Vec<usize>,
    ) {
        if path.len() == target_depth {
            // Build the full topic path
            let mut full_path = base.to_string();
            for component in &path {
                full_path.push_str(&format!("/{:02}", component));
            }
            topics.push(full_path);
            return;
        }

        // Recursively generate for each possible child
        for i in 1..=self.topics_per_node {
            let mut new_path = path.clone();
            new_path.push(i);
            self.generate_at_depth(topics, base, target_depth, new_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_generation() {
        let gen = TopicGenerator::new("test".to_string(), 1, 2, 2);
        let topics = gen.generate_all();

        assert!(topics.contains(&"test00001".to_string()));
        assert!(topics.contains(&"test00001/01".to_string()));
        assert!(topics.contains(&"test00001/02".to_string()));
        assert!(topics.contains(&"test00001/01/01".to_string()));
        assert!(topics.contains(&"test00001/01/02".to_string()));
        assert!(topics.contains(&"test00001/02/01".to_string()));
        assert!(topics.contains(&"test00001/02/02".to_string()));
    }
}
