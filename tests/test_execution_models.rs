pub use q_terminal::{
    bridge, chart_bridge, chart_target, config, contracts_stream, execution, execution_controls,
    execution_models, history, ops_session, ops_status, startup, stream,
};

use execution::store::ExecutionHandle;
use stream::fake_exec as fx;

fn bind_execution_models_handle(
    models: *mut chart_bridge::ExecutionModels,
    handle: ExecutionHandle,
) {
    unsafe {
        let pin = std::pin::Pin::new_unchecked(
            &mut *(models as *mut execution_models::ffi::ExecutionModels),
        );
        use cxx_qt::CxxQtType;
        pin.rust_mut().bind_handle(handle);
    }
}

#[test]
fn test_criterion_3_one_thousand_events_coalesce_to_one_redraw() {
    let _guard = chart_bridge::QT_TEST_MUTEX.lock().unwrap();

    let models = chart_bridge::make_test_execution_models();
    assert!(!models.is_null());

    let handle = ExecutionHandle::new();
    bind_execution_models_handle(models, handle.clone());

    // Initial sync after bind
    unsafe {
        assert_eq!(chart_bridge::execution_models_redraw_count(models), 0);
        assert_eq!(chart_bridge::execution_models_revision(models), 0);

        chart_bridge::execution_models_sync(models);
        assert_eq!(chart_bridge::execution_models_redraw_count(models), 1);
        assert_eq!(chart_bridge::execution_models_revision(models), 1);

        // Calling sync again without events must not bump redraw_count
        chart_bridge::execution_models_sync(models);
        assert_eq!(chart_bridge::execution_models_redraw_count(models), 1);
        assert_eq!(chart_bridge::execution_models_revision(models), 1);
    }

    // Set up a deployment
    let dep_id = "11111111-1111-1111-1111-111111111111";
    handle.mutate(|store| {
        store.apply(
            stream::execution::decode_execution_entry(
                "deployments",
                &fx::deployment(dep_id, "running"),
            )
            .unwrap(),
        );
    });

    // Sync deployment setup frame
    unsafe {
        chart_bridge::execution_models_sync(models);
        assert_eq!(chart_bridge::execution_models_redraw_count(models), 2);
    }

    // Criterion 3 core test:
    // Apply 1 000 events within one frame
    for i in 0..1000 {
        handle.mutate(|store| {
            let order_id = format!("order-{i:05}");
            store.apply(
                stream::execution::decode_execution_entry(
                    "orders",
                    &fx::order(&order_id, dep_id, "submitted"),
                )
                .unwrap(),
            );
        });
    }

    // Before frame sync, redraw_count is still 2
    unsafe {
        assert_eq!(chart_bridge::execution_models_redraw_count(models), 2);

        // Frame boundary arrives: exactly 1 redraw happens
        chart_bridge::execution_models_sync(models);
        assert_eq!(
            chart_bridge::execution_models_redraw_count(models),
            3,
            "1 000 events applied within one frame must cause exactly 1 redraw"
        );
        assert_eq!(chart_bridge::execution_models_revision(models), 3);

        // Second frame boundary with 0 new events: 0 redraws
        chart_bridge::execution_models_sync(models);
        assert_eq!(
            chart_bridge::execution_models_redraw_count(models),
            3,
            "subsequent frame with no events must not redraw"
        );

        // Next frame with 500 events: exactly 1 redraw
        for i in 1000..1500 {
            handle.mutate(|store| {
                let order_id = format!("order-{i:05}");
                store.apply(
                    stream::execution::decode_execution_entry(
                        "orders",
                        &fx::order(&order_id, dep_id, "filled"),
                    )
                    .unwrap(),
                );
            });
        }

        chart_bridge::execution_models_sync(models);
        assert_eq!(
            chart_bridge::execution_models_redraw_count(models),
            4,
            "subsequent frame with events must increment redraw_count by exactly 1"
        );

        chart_bridge::delete_test_execution_models(models);
    }
}
