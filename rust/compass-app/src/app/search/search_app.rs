use super::search_app_result::SearchAppResult;
use crate::{
    app::{app_error::AppError, compass::config::builders::TraversalModelService},
    plugin::input::input_json_extensions::InputJsonExtensions,
};
use chrono::Local;
use compass_core::{
    algorithm::search::{
        a_star::a_star::{backtrack, backtrack_edges, run_a_star, run_a_star_edge_oriented},
        direction::Direction,
    },
    model::{
        frontier::frontier_model::FrontierModel, graphv2::graph::Graph,
        termination::termination_model::TerminationModel,
        traversal::traversal_model::TraversalModel,
    },
    util::read_only_lock::{DriverReadOnlyLock, ExecutorReadOnlyLock},
};
use std::sync::Arc;
use std::time;

pub struct SearchApp {
    graph: Arc<DriverReadOnlyLock<Graph>>,
    traversal_model_service: Arc<DriverReadOnlyLock<Arc<dyn TraversalModelService>>>,
    frontier_model: Arc<DriverReadOnlyLock<Box<dyn FrontierModel>>>,
    termination_model: Arc<DriverReadOnlyLock<TerminationModel>>,
}

impl SearchApp {
    /// builds a new SearchApp from the required components.
    /// handles all of the specialized boxing that allows for simple parallelization.
    pub fn new(
        graph: Graph,
        traversal_model_service: Arc<dyn TraversalModelService>,
        frontier_model: Box<dyn FrontierModel>,
        termination_model: TerminationModel,
    ) -> Self {
        let graph = Arc::new(DriverReadOnlyLock::new(graph));
        let traversal_model_service = Arc::new(DriverReadOnlyLock::new(traversal_model_service));
        let frontier_model = Arc::new(DriverReadOnlyLock::new(frontier_model));
        let termination_model = Arc::new(DriverReadOnlyLock::new(termination_model));
        return SearchApp {
            graph,
            traversal_model_service,
            frontier_model,
            termination_model,
        };
    }

    /// runs a single vertex oriented query
    ///
    pub fn run_vertex_oriented(
        &self,
        query: &serde_json::Value,
    ) -> Result<SearchAppResult, AppError> {
        let o = query.get_origin_vertex().map_err(AppError::PluginError)?;
        let d = query
            .get_destination_vertex()
            .map_err(AppError::PluginError)?;
        let search_start_time = Local::now();
        let dg_inner = Arc::new(self.graph.read_only());

        let tm_inner = self
            .traversal_model_service
            .read_only()
            .read()
            .map_err(|e| AppError::ReadOnlyPoisonError(e.to_string()))?
            .build(query)?;
        let fm_inner = Arc::new(self.frontier_model.read_only());
        let rm_inner = Arc::new(self.termination_model.read_only());
        run_a_star(
            Direction::Forward,
            o,
            d,
            dg_inner,
            tm_inner,
            fm_inner,
            rm_inner,
        )
        .and_then(|tree| {
            let search_end_time = Local::now();
            let search_runtime = (search_end_time - search_start_time)
                .to_std()
                .unwrap_or(time::Duration::ZERO);
            log::debug!(
                "Search Completed in {:?} miliseconds",
                search_runtime.as_millis()
            );
            let route_start_time = Local::now();
            let route = backtrack(o, d, &tree)?;
            let route_end_time = Local::now();
            let route_runtime = (route_end_time - route_start_time)
                .to_std()
                .unwrap_or(time::Duration::ZERO);
            log::debug!(
                "Route Computed in {:?} miliseconds",
                route_runtime.as_millis()
            );
            Ok(SearchAppResult {
                route,
                tree,
                search_runtime,
                route_runtime,
                total_runtime: search_runtime + route_runtime,
            })
        })
        .map_err(AppError::SearchError)
    }

    ///
    /// runs a single edge oriented query
    ///
    pub fn run_edge_oriented(
        &self,
        query: &serde_json::Value,
    ) -> Result<SearchAppResult, AppError> {
        let o = query.get_origin_edge().map_err(AppError::PluginError)?;
        let d = query
            .get_destination_edge()
            .map_err(AppError::PluginError)?;
        let search_start_time = Local::now();
        let dg_inner_search = Arc::new(self.graph.read_only());
        let dg_inner_backtrack = Arc::new(self.graph.read_only());
        let tm_inner = self
            .traversal_model_service
            .read_only()
            .read()
            .map_err(|e| AppError::ReadOnlyPoisonError(e.to_string()))?
            .build(query)?;
        let fm_inner = Arc::new(self.frontier_model.read_only());
        let rm_inner = Arc::new(self.termination_model.read_only());
        run_a_star_edge_oriented(
            Direction::Forward,
            o,
            d,
            dg_inner_search,
            tm_inner,
            fm_inner,
            rm_inner,
        )
        .and_then(|tree| {
            let search_end_time = Local::now();
            let route_start_time = Local::now();
            let route = backtrack_edges(o, d, &tree, dg_inner_backtrack)?;
            let route_end_time = Local::now();
            let search_runtime = (search_end_time - search_start_time)
                .to_std()
                .unwrap_or(time::Duration::ZERO);
            let route_runtime = (route_end_time - route_start_time)
                .to_std()
                .unwrap_or(time::Duration::ZERO);
            Ok(SearchAppResult {
                route,
                tree,
                search_runtime,
                route_runtime,
                total_runtime: search_runtime + route_runtime,
            })
        })
        .map_err(AppError::SearchError)
    }

    /// helper function for accessing the TraversalModel
    ///
    /// example:
    ///
    /// let search_app: SearchApp = ...;
    /// let reference = search_app.get_traversal_model_reference();
    /// let traversal_model = reference.read();
    /// // do things with TraversalModel
    pub fn get_traversal_model_service_reference(
        &self,
    ) -> Arc<ExecutorReadOnlyLock<Arc<dyn TraversalModelService>>> {
        Arc::new(self.traversal_model_service.read_only())
    }

    /// helper function for accessing the TraversalModel
    ///
    /// example:
    ///
    /// let search_app: SearchApp = ...;
    /// let reference = search_app.get_traversal_model_reference();
    /// let traversal_model = reference.read();
    /// // do things with TraversalModel
    pub fn get_traversal_model_reference(
        &self,
        query: &serde_json::Value,
    ) -> Result<Arc<dyn TraversalModel>, AppError> {
        let tm = self
            .traversal_model_service
            .read_only()
            .read()
            .map_err(|e| AppError::ReadOnlyPoisonError(e.to_string()))?
            .build(query)?;
        return Ok(tm);
    }
}
