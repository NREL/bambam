use routee_compass_core::model::traversal::TraversalModelBuilder;
use crate::model::frontier::isochrone::isochrone_frontier_builder::IsochroneFrontierBuilder;
use crate::model::output_plugin::finalize::finalize_output_plugin_builder::FinalizeOutputPluginBuilder;
use crate::model::output_plugin::isochrone::isochrone_output_plugin_builder::IsochroneOutputPluginBuilder;
use crate::model::output_plugin::mep_score::mep_score_plugin_builder::MepScoreOutputPluginBuilder;
use crate::model::output_plugin::opportunity::opportunity_output_plugin_builder::OpportunityOutputPluginBuilder;
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

pub fn bambam_app_builder() -> Result<CompassAppBuilder, CompassAppError> {
    let mut builder = compass_tomtom::builder::tomtom_builder();

    // traversal models
    let fixed_speed_key = String::from("fixed_speed");
    let _scheduled_key = String::from("scheduled");
    let switch_key = String::from("switch");

    // MEP Traversal Models
    let fixed_speed_model: Rc<dyn TraversalModelBuilder> = Rc::new(FixedSpeedBuilder {});

    let switch_model: Rc<dyn TraversalModelBuilder> =
        Rc::new(SwitchTraversalBuilder::new(HashMap::from([])));
    builder.add_traversal_model(fixed_speed_key.clone(), fixed_speed_model);
    builder.add_traversal_model(switch_key.clone(), switch_model.clone());

    // MEP Frontier Models
    let no_restriction: Rc<dyn FrontierModelBuilder> = Rc::new(NoRestrictionBuilder {});
    let road_class: Rc<dyn FrontierModelBuilder> = Rc::new(RoadClassBuilder {});
    let turn_restruction: Rc<dyn FrontierModelBuilder> = Rc::new(TurnRestrictionBuilder {});
    let isochrone_fm = Rc::new(IsochroneFrontierBuilder {});
    let base_frontier_builders: HashMap<String, Rc<dyn FrontierModelBuilder>> = HashMap::from([
        (String::from("no_restriction"), no_restriction),
        (String::from("road_class"), road_class),
        (String::from("turn_restriction"), turn_restruction),
        (String::from("isochrone"), isochrone_fm),
    ]);
    let combined = Rc::new(CombinedFrontierModelBuilder {
        builders: base_frontier_builders.clone(),
    });
    builder.add_frontier_model(String::from("combined"), combined);

    // MEP Input Plugins
    builder.add_input_plugin(String::from("grid"), Rc::new(GridInputPluginBuilder {}));

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
        String::from("mep_score"),
        Rc::new(MepScoreOutputPluginBuilder {}),
    );
    builder.add_output_plugin(
        String::from("finalize"),
        Rc::new(FinalizeOutputPluginBuilder {}),
    );

    Ok(builder)
}
