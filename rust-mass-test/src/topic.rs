/// Generate MQTT topics in a tree structure
///
/// For example with max_depth=2, topics_per_node=3, prefix="test", start_num=1:
/// - test00001
/// - test00001/01, test00001/02, test00001/03
/// - test00001/01/01, test00001/01/02, ..., test00001/03/03

pub struct TopicGenerator {
    prefix: String,
    start_num: usize,
    topics_per_node: usize,
    max_depth: usize,
}

impl TopicGenerator {
    pub fn new(
        prefix: String,
        start_num: usize,
        topics_per_node: usize,
        max_depth: usize,
    ) -> Self {
        TopicGenerator {
            prefix,
            start_num,
            topics_per_node,
            max_depth,
        }
    }

    pub fn generate_all(&self) -> Vec<String> {
        let mut topics = Vec::new();
        let base_topic = format!("{}{:05}", self.prefix, self.start_num);

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

    /// Returns iterator of topic indices for a given base topic and number of topics per branch
    /// This is more efficient than generating all topics upfront
    pub fn topics_iter(&self) -> impl Iterator<Item = String> + '_ {
        TopicIterator::new(
            self.prefix.clone(),
            self.start_num,
            self.topics_per_node,
            self.max_depth,
        )
    }
}

pub struct TopicIterator {
    prefix: String,
    start_num: usize,
    topics_per_node: usize,
    max_depth: usize,
    current_depth_config: Vec<usize>,
    yielded_root: bool,
    done: bool,
}

impl TopicIterator {
    fn new(
        prefix: String,
        start_num: usize,
        topics_per_node: usize,
        max_depth: usize,
    ) -> Self {
        TopicIterator {
            prefix,
            start_num,
            topics_per_node,
            max_depth,
            current_depth_config: Vec::new(),
            yielded_root: false,
            done: false,
        }
    }

    fn next_combination(&mut self) -> bool {
        if self.current_depth_config.is_empty() {
            self.current_depth_config = vec![1; 1];
            return true;
        }

        if self.current_depth_config.len() > self.max_depth {
            return false;
        }

        // Try to increment the last element
        if let Some(last) = self.current_depth_config.last_mut() {
            if *last < self.topics_per_node {
                *last += 1;
                return true;
            }
        }

        // Backtrack and carry over
        let mut pos = self.current_depth_config.len() - 1;
        loop {
            self.current_depth_config[pos] = 1;

            if pos == 0 {
                if self.current_depth_config.len() < self.max_depth {
                    self.current_depth_config.push(1);
                    return true;
                } else {
                    return false;
                }
            }

            pos -= 1;
            if self.current_depth_config[pos] < self.topics_per_node {
                self.current_depth_config[pos] += 1;
                return true;
            }
        }
    }
}

impl Iterator for TopicIterator {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        if self.done {
            return None;
        }

        if !self.yielded_root {
            self.yielded_root = true;
            return Some(format!("{}_{:05}", self.prefix, self.start_num));
        }

        if self.next_combination() {
            let mut topic = format!("{}_{:05}", self.prefix, self.start_num);
            for component in &self.current_depth_config {
                topic.push_str(&format!("/{:02}", component));
            }
            Some(topic)
        } else {
            self.done = true;
            None
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
