#[cfg(test)]
mod query {
    use repository::EqualFilter;
    use repository::LocationRowRepository;
    use repository::mock::mock_store_a;
    use repository::{
        location::{LocationFilter, LocationRepository},
        mock::MockDataInserts,
        test_db::setup_all,
    };

    use crate::{
        location::update::{UpdateLocation, UpdateLocationError},
        service_provider::ServiceProvider,
    };

    #[actix_rt::test]
    async fn location_service_update_errors() {
        let (_, _, connection_manager, _) =
            setup_all("location_service_update_errors", MockDataInserts::all()).await;

        let connection = connection_manager.connection().unwrap();
        let location_repository = LocationRepository::new(&connection);
        let service_provider = ServiceProvider::new(connection_manager);
        let context = service_provider
            .context(mock_store_a().id, "".to_string())
            .unwrap();
        let service = service_provider.location_service;

        let locations_in_store = location_repository
            .query_by_filter(
                LocationFilter::new().store_id(EqualFilter::equal_to("store_a".to_string())),
            )
            .unwrap();

        let locations_not_in_store = location_repository
            .query_by_filter(
                LocationFilter::new().store_id(EqualFilter::not_equal_to("store_a".to_string())),
            )
            .unwrap();

        // Location does not exist
        assert_eq!(
            service.update_location(
                &context,
                UpdateLocation {
                    id: "invalid".to_string(),
                    ..Default::default()
                },
            ),
            Err(UpdateLocationError::LocationDoesNotExist)
        );

        // Location for another store
        assert_eq!(
            service.update_location(
                &context,
                UpdateLocation {
                    id: locations_not_in_store[0].location_row.id.clone(),
                    ..Default::default()
                },
            ),
            Err(UpdateLocationError::LocationDoesNotBelongToCurrentStore)
        );

        // Code used in another store
        assert_eq!(
            service.update_location(
                &context,
                UpdateLocation {
                    id: locations_in_store[0].location_row.id.clone(),
                    code: Some(locations_in_store[1].location_row.code.clone()),
                    ..Default::default()
                },
            ),
            Err(UpdateLocationError::CodeAlreadyExists)
        );
    }
    #[actix_rt::test]
    async fn location_service_update_success() {
        let (_, _, connection_manager, _) =
            setup_all("location_service_update_success", MockDataInserts::all()).await;

        let connection = connection_manager.connection().unwrap();
        let location_repository = LocationRepository::new(&connection);
        let service_provider = ServiceProvider::new(connection_manager);
        let context = service_provider
            .context(mock_store_a().id, "".to_string())
            .unwrap();
        let service = service_provider.location_service;

        let locations_in_store = location_repository
            .query_by_filter(
                LocationFilter::new().store_id(EqualFilter::equal_to("store_a".to_string())),
            )
            .unwrap();

        // Success with no changes
        let location = locations_in_store[0].clone();
        assert_eq!(
            service.update_location(
                &context,
                UpdateLocation {
                    id: location.location_row.id.clone(),
                    ..Default::default()
                },
            ),
            Ok(location.clone())
        );

        assert_eq!(
            location_repository
                .query_by_filter(
                    LocationFilter::new()
                        .id(EqualFilter::equal_to(location.location_row.id.to_string()))
                )
                .unwrap()[0],
            location
        );

        // Success with all changes and code that is not unique accross stores
        let mut location = locations_in_store[1].clone();
        location.location_row.code = "new_location_code".to_string();
        location.location_row.name = "new_location_name".to_string();
        location.location_row.on_hold = !location.location_row.on_hold;

        assert_eq!(
            service.update_location(
                &context,
                UpdateLocation {
                    id: location.location_row.id.clone(),
                    code: Some(location.location_row.code.clone()),
                    name: Some(location.location_row.name.clone()),
                    on_hold: Some(location.location_row.on_hold),
                    ..Default::default()
                },
            ),
            Ok(location.clone())
        );

        assert_eq!(
            location_repository
                .query_by_filter(
                    LocationFilter::new()
                        .id(EqualFilter::equal_to(location.location_row.id.to_string()))
                )
                .unwrap()[0],
            location
        );
    }

    #[actix_rt::test]
    async fn location_service_update_duplicate_code_unchanged() {
        let (_, _, connection_manager, _) = setup_all(
            "location_service_update_duplicate_code_unchanged",
            MockDataInserts::all(),
        )
        .await;

        let connection = connection_manager.connection().unwrap();
        let location_repository = LocationRepository::new(&connection);
        let service_provider = ServiceProvider::new(connection_manager);
        let context = service_provider
            .context(mock_store_a().id, "".to_string())
            .unwrap();
        let service = service_provider.location_service;

        let locations_in_store = location_repository
            .query_by_filter(
                LocationFilter::new().store_id(EqualFilter::equal_to("store_a".to_string())),
            )
            .unwrap();

        // Legacy duplicate (e.g. created via sync, which skips validation)
        let mut duplicate = locations_in_store[1].location_row.clone();
        duplicate.code = locations_in_store[0].location_row.code.clone();
        LocationRowRepository::new(&connection)
            .upsert_one(&duplicate)
            .unwrap();

        // Editing other fields with the unchanged code must still work
        let result = service.update_location(
            &context,
            UpdateLocation {
                id: duplicate.id.clone(),
                code: Some(duplicate.code.clone()),
                on_hold: Some(!duplicate.on_hold),
                ..Default::default()
            },
        );
        assert!(result.is_ok(), "{result:?}");

        // Changing the code to another existing code is still rejected
        assert_eq!(
            service.update_location(
                &context,
                UpdateLocation {
                    id: duplicate.id.clone(),
                    code: Some(locations_in_store[2].location_row.code.clone()),
                    ..Default::default()
                },
            ),
            Err(UpdateLocationError::CodeAlreadyExists)
        );
    }
}
