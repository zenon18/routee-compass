import logging as log
import os
import getpass

import geopandas as gpd
import networkx as nx
import pandas as pd
from sqlalchemy import create_engine
from sqlalchemy.exc import OperationalError

from compass.utils.routee_utils import RouteeModelCollection

log.basicConfig(level=log.INFO)

METERS_TO_MILES = 0.0006213712
KPH_TO_MPH = 0.621371


def add_energy(G: nx.DiGraph) -> nx.DiGraph:
    """
    precompute energy on the graph

    :param G:
    :return:
    """
    routee_model_collection = RouteeModelCollection()

    speed = pd.DataFrame.from_dict(
        nx.get_edge_attributes(G, 'kph'),
        orient="index",
        columns=['gpsspeed'],
    ).multiply(KPH_TO_MPH)
    distance = pd.DataFrame.from_dict(
        nx.get_edge_attributes(G, 'meters'),
        orient="index",
        columns=['miles'],
    ).multiply(METERS_TO_MILES)
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


def build_graph(gdf: gpd.geodataframe.GeoDataFrame) -> nx.DiGraph:
    gdf['id'] = gdf.id.astype(int)
    gdf['f_jnctid'] = gdf.f_jnctid.astype(int)
    gdf['t_jnctid'] = gdf.t_jnctid.astype(int)
    gdf['f_lon'] = gdf.wkb_geometry.apply(lambda g: list(g.coords)[0][0])
    gdf['f_lat'] = gdf.wkb_geometry.apply(lambda g: list(g.coords)[0][1])
    gdf['t_lon'] = gdf.wkb_geometry.apply(lambda g: list(g.coords)[-1][0])
    gdf['t_lat'] = gdf.wkb_geometry.apply(lambda g: list(g.coords)[-1][1])
    oneway_ft = gdf[gdf.oneway == 'FT']
    oneway_tf = gdf[gdf.oneway == 'TF']
    twoway = gdf[~(gdf.oneway == 'FT') & ~(gdf.oneway == 'TF')]

    twoway_edges_tf = [
        (t, f, {
            'meters': mt,
            'minutes': mn,
            'kph': kph,
            'grade': 0
        }) for t, f, mt, mn, kph in zip(
            twoway.t_jnctid.values,
            twoway.f_jnctid.values,
            twoway.meters.values,
            twoway.minutes.values,
            twoway.kph.values,
        )
    ]
    twoway_edges_ft = [
        (f, t, {
            'meters': mt,
            'minutes': mn,
            'kph': kph,
            'grade': 0
        }) for t, f, mt, mn, kph in zip(
            twoway.t_jnctid.values,
            twoway.f_jnctid.values,
            twoway.meters.values,
            twoway.minutes.values,
            twoway.kph.values,
        )
    ]
    oneway_edges_ft = [
        (f, t, {
            'meters': mt,
            'minutes': mn,
            'kph': kph,
            'grade': 0
        }) for t, f, mt, mn, kph in zip(
            oneway_ft.t_jnctid.values,
            oneway_ft.f_jnctid.values,
            oneway_ft.meters.values,
            oneway_ft.minutes.values,
            oneway_ft.kph.values,
        )
    ]
    oneway_edges_tf = [
        (t, f, {
            'meters': mt,
            'minutes': mn,
            'kph': kph,
            'grade': 0
        }) for t, f, mt, mn, kph in zip(
            oneway_tf.t_jnctid.values,
            oneway_tf.f_jnctid.values,
            oneway_tf.meters.values,
            oneway_tf.minutes.values,
            oneway_tf.kph.values,
        )
    ]

    flats = {nid: lat for nid, lat in zip(gdf.f_jnctid.values, gdf.f_lat)}
    flons = {nid: lon for nid, lon in zip(gdf.f_jnctid.values, gdf.f_lon)}
    tlats = {nid: lat for nid, lat in zip(gdf.t_jnctid.values, gdf.t_lat)}
    tlons = {nid: lon for nid, lon in zip(gdf.t_jnctid.values, gdf.t_lon)}

    G = nx.DiGraph()
    G.add_edges_from(twoway_edges_tf)
    G.add_edges_from(twoway_edges_ft)
    G.add_edges_from(oneway_edges_ft)
    G.add_edges_from(oneway_edges_tf)

    nx.set_node_attributes(G, flats, "lat")
    nx.set_node_attributes(G, flons, "lon")
    nx.set_node_attributes(G, tlats, "lat")
    nx.set_node_attributes(G, tlons, "lon")

    log.info("extracting largest connected component..")
    n_edges_before = G.number_of_edges()
    G = nx.DiGraph(G.subgraph(max(nx.strongly_connected_components(G), key=len)))
    n_edges_after = G.number_of_edges()
    log.info(f"final graph has {n_edges_after} edges, lost {n_edges_before - n_edges_after}")

    return G


if __name__ == "__main__":
    username = input("Please enter your Trolley username: ")
    password = getpass.getpass("Please enter your Trolley password: ")
    try:
        engine = create_engine('postgresql://' + username + ':' + password + '@trolley.nrel.gov:5432/master')
        engine.connect()
        log.info("established connection with Trolley")
    except OperationalError as oe:
        raise IOError("can't connect to Trolley..") from oe

    shp_file = os.path.join("denver_metro", "denver_metro.shp")
    denver_gdf = gpd.read_file(shp_file)
    denver_polygon = denver_gdf.iloc[0].geometry

    log.info("pulling raw tomtom network from Trolley..")
    q = f"""
    select id, f_jnctid, t_jnctid, frc, backrd, rdcond, privaterd, roughrd, meters, minutes, kph, oneway, wkb_geometry 
    from tomtom_multinet_2017.multinet_2017 as mn
    where ST_Contains(ST_GeomFromEWKT('SRID={denver_gdf.crs.to_epsg()};{denver_polygon.wkt}'), 
    ST_GeomFromEWKB(mn.wkb_geometry))
    """
    raw_gdf = gpd.GeoDataFrame.from_postgis(
        q,
        con=engine,
        geom_col="wkb_geometry",
    )
    log.info(f"pulled {raw_gdf.shape[0]} links")

    log.info("filtering out bad links..")
    raw_gdf = raw_gdf[
        (raw_gdf.rdcond == 1) &
        (raw_gdf.frc < 7) &
        (raw_gdf.backrd == 0) &
        (raw_gdf.privaterd == 0) &
        (raw_gdf.roughrd == 0)
        ]
    log.info(f"{raw_gdf.shape[0]} links remain after filtering")

    log.info("building graph from raw network..")
    G = build_graph(raw_gdf)

    log.info("precomputing energy on the network..")
    G = add_energy(G)

    log.info("writing to file..")
    path = os.path.join("..", "resources", "denver_metro_tomtom_roadnetwork.pickle")
    nx.write_gpickle(G, path)