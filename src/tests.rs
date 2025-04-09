use crate::{bucket::Bucket, watch_path::WatchPath, *};

#[test]
fn parse_config() {
    let input = "
        [[watch]]
        path = \"/some/path\"
        recursive_mode = \"non-recursive\"
        bucket_names = [\"bucket1\", \"bucket2\", \"bucket3\"]

        [[bucket]]
        name = \"bucket1\"
        destination = \"/other/path\"
        extension_filters = [\"zip\"]
        name_filters = [\".*\\\\.tar\\\\.gz\"]
        action = \"copy\"
        priority = 0
        override_action = \"skip\"

        [[bucket]]
        name = \"bucket2\"
        destination = \"/other/other/path\"
        extension_filters = [\"exe\", \"bin\"]
        name_filters = []
        action = \"move\"
        override_action = \"rename\"
        priority = 0

        [[bucket]]
        name = \"bucket3\"
        destination = \"/random/path\"
        extension_filters = [\"obj\"]
        name_filters = []
        action = \"delete\"
        priority = 255
        override_action = \"overwrite\"
        ";

    let exp = Config {
        watch: Vec::from([WatchPath {
            path: "/some/path".into(),
            recursive_mode: watch_path::RecMode::NonRecursive,
            bucket_names: vec!["bucket1".into(), "bucket2".into(), "bucket3".into()],
        }]),
        bucket: Vec::from([
            Bucket {
                name: "bucket1".into(),
                destination: "/other/path".into(),
                extension_filters: vec!["zip".into()],
                name_filters: vec![".*\\.tar\\.gz".into()],
                priority: 0,
                action: bucket::Action::Copy,
                override_action: Default::default(),
                _regexes: Vec::new(),
            },
            Bucket {
                name: "bucket2".into(),
                destination: "/other/other/path".into(),
                extension_filters: vec!["exe".into(), "bin".into()],
                name_filters: vec![],
                priority: 0,
                action: bucket::Action::Move,
                override_action: bucket::OverrideAction::Rename,
                _regexes: Vec::new(),
            },
            Bucket {
                name: "bucket3".into(),
                destination: "/random/path".into(),
                extension_filters: vec!["obj".into()],
                name_filters: vec![],
                priority: 255,
                action: bucket::Action::Delete,
                override_action: bucket::OverrideAction::Overwrite,
                _regexes: Vec::new(),
            },
        ]),
    };

    let res = toml::from_str(input);

    assert_eq!(res, Ok(exp));
}
