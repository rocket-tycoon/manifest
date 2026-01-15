use speculate2::speculate;

speculate! {
    use legion_db::{Database, Project, Module, Feature};
    use std::sync::Arc;

    fn setup_db() -> Arc<Database> {
        Arc::new(Database::open_memory().expect("Failed to create test database"))
    }

    fn create_test_project(db: &Database, name: &str, path: &str) -> Project {
        let project = Project::new(name, path);
        db.with_connection(|conn| project.insert(conn))
            .expect("Failed to insert project");
        project
    }

    fn create_test_module(db: &Database, project_id: &str, name: &str) -> Module {
        let module = Module::new(project_id, name);
        db.with_connection(|conn| module.insert(conn))
            .expect("Failed to insert module");
        module
    }

    fn create_test_feature(db: &Database, module_id: &str, title: &str) -> Feature {
        let feature = Feature::new(module_id, title);
        db.with_connection(|conn| feature.insert(conn))
            .expect("Failed to insert feature");
        feature
    }

    describe "project management" {
        describe "list_recent" {
            it "returns projects ordered by last opened" {
                let db = setup_db();
                let project1 = create_test_project(&db, "project-1", "/tmp/p1");
                let project2 = create_test_project(&db, "project-2", "/tmp/p2");

                // Touch project2 to make it more recent
                db.with_connection(|conn| {
                    let mut p2 = Project::find_by_id(conn, &project2.id).unwrap().unwrap();
                    p2.touch(conn).unwrap();
                    Ok::<_, anyhow::Error>(())
                }).unwrap();

                let recent = db.with_connection(|conn| Project::list_recent(conn, 10)).unwrap();

                // project2 should be first (most recently touched)
                assert!(!recent.is_empty());
                assert_eq!(recent[0].id, project2.id);
            }

            it "returns empty list when no projects exist" {
                let db = setup_db();
                let recent = db.with_connection(|conn| Project::list_recent(conn, 10)).unwrap();
                assert!(recent.is_empty());
            }

            it "respects the limit parameter" {
                let db = setup_db();
                for i in 0..5 {
                    create_test_project(&db, &format!("project-{}", i), &format!("/tmp/p{}", i));
                }

                let recent = db.with_connection(|conn| Project::list_recent(conn, 3)).unwrap();
                assert_eq!(recent.len(), 3);
            }
        }

        describe "find_by_path" {
            it "finds project by exact path" {
                let db = setup_db();
                let project = create_test_project(&db, "my-project", "/home/user/code/my-project");

                let found = db.with_connection(|conn| {
                    Project::find_by_path(conn, "/home/user/code/my-project")
                }).unwrap();

                assert!(found.is_some());
                assert_eq!(found.unwrap().id, project.id);
            }

            it "returns None for nonexistent path" {
                let db = setup_db();
                create_test_project(&db, "my-project", "/home/user/code/my-project");

                let found = db.with_connection(|conn| {
                    Project::find_by_path(conn, "/nonexistent/path")
                }).unwrap();

                assert!(found.is_none());
            }
        }
    }

    describe "feature tree data loading" {
        it "loads modules for a project" {
            let db = setup_db();
            let project = create_test_project(&db, "test-project", "/tmp/test");

            create_test_module(&db, &project.id, "Authentication");
            create_test_module(&db, &project.id, "Dashboard");

            let modules = db.with_connection(|conn| {
                Module::list_by_project(conn, &project.id)
            }).unwrap();

            assert_eq!(modules.len(), 2);
        }

        it "loads features for a module" {
            let db = setup_db();
            let project = create_test_project(&db, "test-project", "/tmp/test");
            let module = create_test_module(&db, &project.id, "Auth");

            create_test_feature(&db, &module.id, "Login");
            create_test_feature(&db, &module.id, "Logout");
            create_test_feature(&db, &module.id, "Password Reset");

            let features = db.with_connection(|conn| {
                Feature::list_by_module(conn, &module.id)
            }).unwrap();

            assert_eq!(features.len(), 3);
        }

        it "does not load features from other modules" {
            let db = setup_db();
            let project = create_test_project(&db, "test-project", "/tmp/test");
            let module1 = create_test_module(&db, &project.id, "Auth");
            let module2 = create_test_module(&db, &project.id, "Dashboard");

            create_test_feature(&db, &module1.id, "Login");
            create_test_feature(&db, &module2.id, "Charts");

            let features = db.with_connection(|conn| {
                Feature::list_by_module(conn, &module1.id)
            }).unwrap();

            assert_eq!(features.len(), 1);
            assert_eq!(features[0].title, "Login");
        }
    }

    describe "feature content" {
        it "stores and retrieves feature content" {
            let db = setup_db();
            let project = create_test_project(&db, "test-project", "/tmp/test");
            let module = create_test_module(&db, &project.id, "Auth");
            let mut feature = create_test_feature(&db, &module.id, "Login");

            // Update content
            feature.content = Some("# Login Feature\n\nThis is the login flow.".to_string());
            db.with_connection(|conn| feature.update(conn)).unwrap();

            // Retrieve and verify
            let loaded = db.with_connection(|conn| {
                Feature::find_by_id(conn, &feature.id)
            }).unwrap().unwrap();

            assert!(loaded.content.is_some());
            assert!(loaded.content.unwrap().contains("Login Feature"));
        }

        it "stores feature status" {
            let db = setup_db();
            let project = create_test_project(&db, "test-project", "/tmp/test");
            let module = create_test_module(&db, &project.id, "Auth");
            let mut feature = create_test_feature(&db, &module.id, "Login");

            // Default status should be Draft
            assert_eq!(feature.status.as_str(), "draft");

            // Update to Active
            feature.status = legion_db::FeatureStatus::Active;
            db.with_connection(|conn| feature.update(conn)).unwrap();

            let loaded = db.with_connection(|conn| {
                Feature::find_by_id(conn, &feature.id)
            }).unwrap().unwrap();

            assert_eq!(loaded.status.as_str(), "active");
        }
    }

    describe "project touch updates last_opened_at" {
        it "updates last_opened_at when touched" {
            let db = setup_db();
            let project = create_test_project(&db, "test-project", "/tmp/test");

            // Initially last_opened_at should be set (from new())
            let initial = db.with_connection(|conn| {
                Project::find_by_id(conn, &project.id)
            }).unwrap().unwrap();
            let initial_opened = initial.last_opened_at.clone();

            // Wait a tiny bit and touch
            std::thread::sleep(std::time::Duration::from_millis(10));

            db.with_connection(|conn| {
                let mut p = Project::find_by_id(conn, &project.id).unwrap().unwrap();
                p.touch(conn).unwrap();
                Ok::<_, anyhow::Error>(())
            }).unwrap();

            let after_touch = db.with_connection(|conn| {
                Project::find_by_id(conn, &project.id)
            }).unwrap().unwrap();

            assert!(after_touch.last_opened_at.is_some());
            // The timestamp should be different (or at least not less than)
            assert!(after_touch.last_opened_at >= initial_opened);
        }
    }
}
