from typing import Tuple

import networkx as nx

# TODO: maybe we can remove pandas dependency for prototype if we precompute energy -ndr
import pandas as pd

from rtree import index

from compass.road_network.base import RoadNetwork, PathWeight
from compass.utils.geo_utils import Coordinate
from compass.utils.routee_utils import RouteeModelCollection


class OSMRoadNetwork(RoadNetwork):
    """
    osm road network
    """
    network_weights = {
        PathWeight.DISTANCE: "miles",
        PathWeight.TIME: "travel_time",
        PathWeight.ENERGY: "energy"
    }

    def __init__(
            self,
            osm_network_file: str,
            routee_model_collection: RouteeModelCollection = RouteeModelCollection(),
    ):
        self.G = nx.read_gpickle(osm_network_file)
        self.rtree = self._build_rtree()

        self.routee_model_collection = routee_model_collection

    def _compute_energy(self):
        """
        computes energy over the road network for all routee models in the routee model collection.

        this isn't currently called by anything since we're pre-computing energy for the prototype but
        would presumably be called if we want to do live updates.
        """

        speed = pd.DataFrame.from_dict(
            nx.get_edge_attributes(self.G, 'speed_mph'),
            orient="index",
            columns=['gpsspeed'],
        )
        distance = pd.DataFrame.from_dict(
            nx.get_edge_attributes(self.G, 'miles'),
            orient="index",
            columns=['miles'],
        )
        grade = pd.DataFrame.from_dict(
            nx.get_edge_attributes(self.G, 'grade'),
            orient="index",
            columns=['grade'],
        )
        df = speed.join(distance).join(grade)

        for k, model in self.routee_model_collection.routee_models.items():
            energy = model.predict(df).to_dict()
            nx.set_edge_attributes(self.G, name=f"{self.network_weights[PathWeight.ENERGY]}_{k}", values=energy)

    def _build_rtree(self) -> index.Index:
        tree = index.Index()
        for nid in self.G.nodes():
            lat = self.G.nodes[nid]['y']
            lon = self.G.nodes[nid]['x']
            tree.insert(nid, (lat, lon, lat, lon))

        return tree

    def _get_nearest_node(self, coord: Coordinate) -> str:
        node_id = list(self.rtree.nearest((coord.lat, coord.lon, coord.lat, coord.lon), 1))[0]

        return node_id

    def shortest_path(
            self,
            origin: Coordinate,
            destination: Coordinate,
            weight: PathWeight = PathWeight.DISTANCE,
            routee_key: str = "Gasoline",
    ) -> Tuple[Coordinate, ...]:
        """
        computes weighted shortest path
        :return: shortest path as series of coordinates
        """
        origin_id = self._get_nearest_node(origin)
        dest_id = self._get_nearest_node(destination)

        network_weight = self.network_weights[weight]

        if weight == PathWeight.ENERGY:
            network_weight += f"_{routee_key}"

        nx_route = nx.shortest_path(
            self.G,
            origin_id,
            dest_id,
            weight=self.network_weights[weight],
        )

        route = tuple(Coordinate(lat=self.G.nodes[n]['y'], lon=self.G.nodes[n]['x']) for n in nx_route)

        return route