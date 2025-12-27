27,36c\
    fn create_test_entry(id: usize, timestamp: &str, level: &str, file: &str, line: usize, content: &str) -> LogEntry {\
        LogEntry {\
            id,\
            timestamp: timestamp.to_string(),\
            level: level.to_string(),\
            file: file.to_string(),\
            line,\
            real_path: format!("cas://hash{}", id),\
            content: content.to_string(),\
            tags: vec![],\
            match_details: None,\
            matched_keywords: None,\
        }\
    }
