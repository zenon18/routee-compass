import logging as log
import os

import geopandas as gpd
import networkx as nx
import osmnx as ox
import pandas as pd

from compass.utils.routee_utils import RouteeModelCollection

ox.config(log_console=True)
log.basicConfig(level=log.INFO)

DEFAULT_MPH = 30
_unit_conversion = {
    'mph': 1,
    'kmph': 0.621371,
}
METERS_TO_MILES = 0.0006213712


def parse_road_network_graph(g):
    osm_speed = nx.get_edge_attributes(g, 'speed_kph')
    speed_mph = {k: v * _unit_conversion['kmph'] for k, v in osm_speed.items()}
    nx.set_edge_attributes(g, speed_mph, 'speed_mph')

    length_meters = nx.get_edge_attributes(g, 'length')
    length_miles = {k: v * METERS_TO_MILES for k, v in length_meters.items()}
    nx.set_edge_attributes(g, length_miles, 'miles')

    # TODO add real grade here
    nx.set_edge_attributes(g, name="grade", values=0)

    return g


def compress(G):
    """
    a hacky way to delete unnecessary data on the networkx graph

    :param G: graph to be compressed
    :return: compressed graph
    """
    keys_to_delete = [
        'oneway',
        'ref',
        'access',
        'lanes',
        'name',
        'maxspeed',
        'highway',
        'length',
        'geometry',
        'speed_kph',
        'osmid'
    ]

    for _, _, d in G.edges(data=True):
        for k in keys_to_delete:
            try:
                del d[k]
            except KeyError:
                continue

    for _, d in G.nodes(data=True):
        for k in keys_to_delete:
            try:
                del d[k]
            except KeyError:
                continue

    return G


def add_energy(G):
    """
    precompute energy on the graph

    :param G:
    :return:
    """
    routee_model_collection = RouteeModelCollection()

    speed = pd.DataFrame.from_dict(
        nx.get_edge_attributes(G, 'speed_mph'),
        orient="index",
        columns=['gpsspeed'],
    )
    distance = pd.DataFrame.from_dict(
        nx.get_edge_attributes(G, 'miles'),
        orient="index",
        columns=['miles'],
    )
    grade = pd.DataFrame.from_dict(
        nx.get_edge_attributes(G, 'grade'),
        orient="index",
        columns=['grade'],
    )
    df = speed.join(distance).join(grade)

    for k, model in routee_model_collection.routee_models.items():
        energy = model.predict(df).to_dict()
        nx.set_edge_attributes(G, name=f"energy_{k}", values=energy)

    return G


if __name__ == "__main__":
    shp_file = os.path.join("denver_metro", "denver_metro.shp")
    denver_gdf = gpd.read_file(shp_file)
    denver_polygon = denver_gdf.iloc[0].geometry

    log.info("pulling raw osm network..")
    G = ox.graph_from_polygon(denver_polygon, network_type="drive")

    log.info("parsing speeds and computing travel times..")
    G = ox.add_edge_speeds(G)
    G = ox.add_edge_travel_times(G)
    G = parse_road_network_graph(G)

    log.info("computing largest strongly connected component..")
    # this makes sure there are no graph 'dead-ends'
    G = ox.utils_graph.get_largest_component(G, strongly=True)

    log.info("pre-computing energy..")
    G = add_energy(G)

    log.info("compressing..")
    G = compress(G)

    # recreating the graph as a workaround to remove a shapely dependency
    outg = nx.MultiDiGraph()
    outg.add_nodes_from(G.nodes(data=True))
    outg.add_edges_from(G.edges(data=True, keys=True))

    log.info("writing to file..")
    path = os.path.join("..", "resources", "denver_metro_osm_roadnetwork.pickle")
    nx.write_gpickle(outg, path)