use crate::model::frontier::multimodal::MultimodalFrontierBuilder;
use crate::model::frontier::time_limit::TimeLimitFrontierBuilder;
use crate::model::input_plugin::grid_geometry::grid_geometry_input_plugin::GridGeometryInputPlugin;
use crate::model::input_plugin::grid_geometry::grid_geometry_input_plugin_builder::GridGeometryInputPluginBuilder;
use crate::model::output_plugin::finalize::finalize_output_plugin_builder::FinalizeOutputPluginBuilder;
use crate::model::output_plugin::isochrone::isochrone_output_plugin_builder::IsochroneOutputPluginBuilder;
use crate::model::output_plugin::opportunity::OpportunityOutputPluginBuilder;
use crate::model::traversal::transit::TransitTraversalBuilder;
use crate::model::traversal::multimodal::MultimodalTraversalBuilder;
use crate::model::traversal::switch::switch_traversal_builder::SwitchTraversalBuilder;
use inventory;
use routee_compass::app::compass::BuilderRegistration;
use routee_compass::app::compass::CompassAppError;
use routee_compass_core::model::frontier::FrontierModelBuilder;
use routee_compass_core::model::traversal::TraversalModelBuilder;
use std::collections::HashMap;
use std::rc::Rc;

use super::input_plugin::grid::grid_input_plugin_builder::GridInputPluginBuilder;
use super::traversal::fixed_speed::FixedSpeedBuilder;
use super::traversal::time_delay::TripArrivalDelayBuilder;
use super::traversal::time_delay::TripDepartureDelayBuilder;

/// builders to inject into the CompassBuilderInventory on library load via the inventory crate
pub const BUILDER_REGISTRATION: BuilderRegistration = BuilderRegistration(|builders| {
    builders.add_traversal_model(String::from("fixed_speed"), Rc::new(FixedSpeedBuilder {}));
    builders.add_traversal_model(
        String::from("departure"),
        Rc::new(TripDepartureDelayBuilder {}),
    );
    builders.add_traversal_model(String::from("arrival"), Rc::new(TripArrivalDelayBuilder {}));
    builders.add_traversal_model(
        String::from("multimodal"),
        Rc::new(MultimodalTraversalBuilder {}),
    );

    builders.add_traversal_model(
        String::from("transit"),
        Rc::new(TransitTraversalBuilder {}),
    );

    builders.add_frontier_model(
        "multimodal".to_string(),
        Rc::new(MultimodalFrontierBuilder {}),
    );
    builders.add_frontier_model(
        String::from("time_limit"),
        Rc::new(TimeLimitFrontierBuilder {}),
    );

    builders.add_input_plugin(String::from("grid"), Rc::new(GridInputPluginBuilder {}));
    builders.add_input_plugin(
        String::from("grid_geometry"),
        Rc::new(GridGeometryInputPluginBuilder {}),
    );

    builders.add_output_plugin(
        String::from("isochrone"),
        Rc::new(IsochroneOutputPluginBuilder {}),
    );
    builders.add_output_plugin(
        String::from("opportunity"),
        Rc::new(OpportunityOutputPluginBuilder {}),
    );
    builders.add_output_plugin(
        String::from("finalize"),
        Rc::new(FinalizeOutputPluginBuilder {}),
    );
    Ok(())
});
