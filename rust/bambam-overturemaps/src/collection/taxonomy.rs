use csv::ReaderBuilder;
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::collection::error::OvertureMapsCollectionError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaxonomyModelBuilder {
    activity_mappings: HashMap<String, Vec<String>>,
    source_url: Option<String>,
}

impl From<HashMap<String, Vec<String>>> for TaxonomyModelBuilder {
    fn from(value: HashMap<String, Vec<String>>) -> Self {
        Self {
            activity_mappings: value,
            source_url: None,
        }
    }
}

impl TaxonomyModelBuilder {
    pub fn new(
        activity_mappings: HashMap<String, Vec<String>>,
        source_url: Option<String>,
    ) -> Self {
        Self {
            activity_mappings,
            source_url,
        }
    }

    pub fn build(&self) -> Result<TaxonomyModel, OvertureMapsCollectionError> {
        // Collect taxonomy records from CSV
        // Create a new thread to handle async operations
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                OvertureMapsCollectionError::TaxonomyLoadingError(format!(
                    "Failed to safely create thread to consume taxonomy csv: {e}"
                ))
            })?;
        let taxonomy_tree = runtime.block_on(parse_overturemaps_places_taxonomy_csv(
            self.source_url.clone(),
        ))?;

        // Process records to identify (child, parent) pairs
        let processed_tree_nodes = taxonomy_tree
            .into_iter()
            .map(|(category, parents)| {
                if parents.len() < 2 {
                    return (category, None);
                };
                (category, Some(parents[parents.len() - 2].to_owned()))
            })
            .collect::<Vec<(String, Option<String>)>>();

        // Cloning here too expensive?
        Ok(TaxonomyModel::from_tree_nodes(
            processed_tree_nodes,
            self.activity_mappings.clone(),
        ))
    }

    pub fn get_mappings(&self) -> HashMap<String, Vec<String>> {
        self.activity_mappings.clone()
    }
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct TaxonomyCSVRecord {
    #[serde(rename = "Category code")]
    category: String,
    #[serde(rename = "Overture Taxonomy")]
    taxonomy: String,
}

async fn parse_overturemaps_places_taxonomy_csv(
    source_url: Option<String>,
) -> Result<Vec<(String, Vec<String>)>, OvertureMapsCollectionError> {
    // Set default for the url
    let source_url = source_url.unwrap_or(String::from("https://raw.githubusercontent.com/OvertureMaps/schema/refs/heads/main/docs/schema/concepts/by-theme/places/overture_categories.csv"));

    // Execute GET request and parse response as text
    let response = reqwest::get(source_url).await.map_err(|e| {
        OvertureMapsCollectionError::TaxonomyLoadingError(format!("GET request failed: {e}"))
    })?;
    let csv_content = response.text().await.map_err(|e| {
        OvertureMapsCollectionError::TaxonomyLoadingError(format!("Parsing response failed: {e}"))
    })?;

    // Parse text from response as CSV
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b';')
        .trim(csv::Trim::All)
        .from_reader(csv_content.as_bytes());

    // Deserialize each row into Taxonomy record and then into (String, Vec<String>)
    let mut results = Vec::new();
    for result in rdr.deserialize() {
        let record: TaxonomyCSVRecord = result
            .map_err(|e| OvertureMapsCollectionError::TaxonomyDeserializingError(format!("{e}")))?;
        let taxonomy: Vec<String> = record
            .taxonomy
            .replace("[", "")
            .replace("]", "")
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        results.push((record.category, taxonomy));
    }

    Ok(results)
}

/// Implements the logic of grouping OvertureMaps Labels
/// into semantically equivalent groups
/// e.g.
///   restaurant, bar -> food
///   sports_and_recreation_venue, arts_and_entertainment -> entertainment
///
/// It internally uses a `CategoryTree` to obtain a map from group label
/// to OvertureMaps label with constant time complexity
#[derive(Debug, Clone)]
pub struct TaxonomyModel {
    // group_labels: Vec<String>,
    group_mappings: HashMap<String, HashSet<String>>,
}

impl TaxonomyModel {
    pub fn from_tree_nodes(
        tree_nodes: Vec<(String, Option<String>)>,
        group_mappings: HashMap<String, Vec<String>>,
    ) -> Self {
        // Build category tree with nodes
        let category_tree = CategoryTree::new(tree_nodes);

        // Linearize each query
        let activity_mappings = group_mappings
            .into_iter()
            .map(|(activity, categories)| {
                (activity, category_tree.get_linearized_query(categories))
            })
            .collect::<HashMap<String, HashSet<String>>>();

        TaxonomyModel {
            // group_labels: activity_fields,
            group_mappings: activity_mappings,
        }
    }

    /// This constructor receives a complete mapping, it does not expand
    /// from a CategoryTree
    pub fn from_mapping(mapping: HashMap<String, HashSet<String>>) -> Self {
        Self {
            group_mappings: mapping,
        }
    }

    /// Compute the union of all mappings. Useful to filter points as those are retrieved from an external source.
    pub fn get_unique_categories(&self) -> HashSet<String> {
        let mut result = HashSet::<String>::new();
        for set in self
            .group_mappings
            .values()
            .cloned()
            .collect::<Vec<HashSet<String>>>()
        {
            result.extend(set);
        }
        result
    }

    pub fn reverse_map(
        &self,
        categories: &Vec<String>,
        group_labels: Vec<String>,
    ) -> Result<Vec<Vec<bool>>, OvertureMapsCollectionError> {
        categories
            .iter()
            .map(|category| {
                group_labels
                    .iter()
                    .map(|group| {
                        Ok::<bool, OvertureMapsCollectionError>(
                            self.group_mappings
                                .get(group)
                                .ok_or(OvertureMapsCollectionError::GroupMappingError(format!(
                                    "Group {group} was not found in mapping"
                                )))?
                                .contains(category),
                        )
                    })
                    .collect::<Result<Vec<bool>, OvertureMapsCollectionError>>()
            })
            .collect::<Result<Vec<Vec<bool>>, OvertureMapsCollectionError>>()
    }
}

#[derive(Default, Debug)]
struct CategoryTree(HashMap<String, Vec<String>>);

impl CategoryTree {
    /// Creates a new CategoryTree data structure from a vector of category -> parent relationships.
    /// The final datastructure is a HashMap where the keys are nodes and the values are vectors of
    /// Strings representing the children of each node.
    ///
    /// # Arguments
    ///
    /// * `nodes` - Vector of cateogry, parent node relationships. All nodes are
    ///             expected to be represented by strings. If no parent is given,
    ///             i.e. the second element is None, the entry is ignored
    fn new(nodes: Vec<(String, Option<String>)>) -> Self {
        let mut tree: HashMap<String, Vec<String>> = HashMap::new();

        for node in nodes {
            let (category, parent) = node;

            // If parent is None, we ignore this entry
            if let Some(parent_label) = parent {
                let parent_node = tree.entry(parent_label.clone()).or_default();
                parent_node.push(category);
            }
            // We only need to take care of the second to last one
            // because the every entry in this list is a node that eventually
            // will be processed
            // if parents.len() < 2 {continue;}

            // let parent_label = &parents[parents.len() - 2];
            // let parent_node = tree.entry(parent_label.clone()).or_insert(vec![]);
            // parent_node.push(category);
        }

        Self(tree)
    }

    /// Recursively get a linear representation of all the nodes below a given node in the tree. The
    /// output of this function includes the node itself. If the node is not found, it returns an empty list.
    ///
    /// # Arguments
    ///
    /// * `node` - Node at which to start the search.
    fn get_linearized_children(&self, node: String) -> Vec<String> {
        let mut node_children = self.0.get(&node).cloned().unwrap_or(Vec::<String>::new());
        let recursive_children: Vec<String> = node_children
            .iter()
            .flat_map(|e| self.get_linearized_children(e.to_owned()))
            .collect();
        node_children.extend(recursive_children);
        node_children
    }

    /// Compute a linear representation of all the possible values that would satisfy a query. In this case,
    /// a query is a vector of nodes in the tree and all the nodes below them. The ouput of this function
    /// is a HashSet that contains all the possible values in the original input query and their recursive
    /// children.
    /// # Arguments
    ///
    /// * `query` - Vector of all the categories to be considered
    fn get_linearized_query(&self, query: Vec<String>) -> HashSet<String> {
        // Linearize the query: get all the possible values that match
        // e.g If I put restaurant, any of restaurant, afghan_restaurant, moroccan_restaurant, ... work
        let linearized_children: Vec<String> = query
            .iter()
            .flat_map(|e| self.get_linearized_children(e.to_owned()))
            .collect();

        // Make a HashSet with the linearized query
        HashSet::from_iter(query.into_iter().chain(linearized_children))
    }
}
