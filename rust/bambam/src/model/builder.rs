use routee_compass_core::model::traversal::TraversalModelBuilder;
use crate::model::frontier::time_limit::TimeLimitFrontierBuilder;
use crate::model::input_plugin::grid_geometry::grid_geometry_input_plugin::GridGeometryInputPlugin;
use crate::model::input_plugin::grid_geometry::grid_geometry_input_plugin_builder::GridGeometryInputPluginBuilder;
use crate::model::output_plugin::finalize::finalize_output_plugin_builder::FinalizeOutputPluginBuilder;
use crate::model::output_plugin::isochrone::isochrone_output_plugin_builder::IsochroneOutputPluginBuilder;
use crate::model::output_plugin::opportunity::opportunity_output_plugin_builder::OpportunityOutputPluginBuilder;
use crate::model::traversal::multimodal::MultimodalTraversalBuilder;
use crate::model::traversal::switch::switch_traversal_builder::SwitchTraversalBuilder;
use routee_compass::app::compass::CompassAppError;
use routee_compass::app::compass::CompassAppBuilder;
use routee_compass::app::compass::model::frontier_model::combined::combined_builder::CombinedFrontierModelBuilder;
use routee_compass::app::compass::model::frontier_model::no_restriction_builder::NoRestrictionBuilder;
use routee_compass::app::compass::model::frontier_model::road_class::road_class_builder::RoadClassBuilder;
use routee_compass::app::compass::model::frontier_model::turn_restrictions::turn_restriction_builder::TurnRestrictionBuilder;
use routee_compass_core::model::frontier::FrontierModelBuilder;
use std::collections::HashMap;
use std::rc::Rc;

use super::input_plugin::grid::grid_input_plugin_builder::GridInputPluginBuilder;
use super::traversal::fixed_speed::FixedSpeedBuilder;
use super::traversal::time_delay::TripArrivalDelayBuilder;
use super::traversal::time_delay::TripDepartureDelayBuilder;

pub fn bambam_app_builder() -> Result<CompassAppBuilder, CompassAppError> {
    let mut builder = compass_tomtom::builder::tomtom_builder();

    // MEP Traversal Models
    let fixed_speed_model: Rc<dyn TraversalModelBuilder> = Rc::new(FixedSpeedBuilder {});
    let departure_model: Rc<dyn TraversalModelBuilder> = Rc::new(TripDepartureDelayBuilder {});
    let arrival_model: Rc<dyn TraversalModelBuilder> = Rc::new(TripArrivalDelayBuilder {});
    let multimodal_model: Rc<dyn TraversalModelBuilder> = Rc::new(MultimodalTraversalBuilder {});
    builder.add_traversal_model(String::from("fixed_speed"), fixed_speed_model);
    builder.add_traversal_model(String::from("departure"), departure_model);
    builder.add_traversal_model(String::from("arrival"), arrival_model);
    builder.add_traversal_model(String::from("multimodal"), multimodal_model);

    // MEP Frontier Models
    let isochrone_fm = Rc::new(TimeLimitFrontierBuilder {});
    builder.add_frontier_model(String::from("time_limit"), isochrone_fm);

    // MEP Input Plugins
    builder.add_input_plugin(String::from("grid"), Rc::new(GridInputPluginBuilder {}));
    builder.add_input_plugin(
        String::from("grid_geometry"),
        Rc::new(GridGeometryInputPluginBuilder {}),
    );

    // MEP Output Plugins
    builder.add_output_plugin(
        String::from("isochrone"),
        Rc::new(IsochroneOutputPluginBuilder {}),
    );
    builder.add_output_plugin(
        String::from("opportunity"),
        Rc::new(OpportunityOutputPluginBuilder {}),
    );
    builder.add_output_plugin(
        String::from("finalize"),
        Rc::new(FinalizeOutputPluginBuilder {}),
    );

    Ok(builder)
}
