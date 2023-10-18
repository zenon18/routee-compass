from enum import Enum
from pathlib import Path

import math
import itertools
import logging

log = logging.getLogger(__name__)


class TileResolution(Enum):
    ONE_ARC_SECOND = 1
    ONE_THIRD_ARC_SECOND = 13
    ONE_NINTH_ARC_SECOND = 19


def cover_floats_with_integers(float_min: float, float_max: float) -> list[int]:
    if float_max < float_min:
        raise ValueError("float max must be greater than float min")

    start = math.floor(float_min)
    end = math.ceil(float_max)

    integers = list(range(start, end + 1))
    return integers


def lat_lon_to_tile(coord: tuple[int, int]) -> str:
    lat, lon = coord
    if lat < 0:
        lat_prefix = "s"
    else:
        lat_prefix = "n"
    if lon < 0:
        lon_prefix = "w"
    else:
        lon_prefix = "e"

    return f"{lat_prefix}{abs(lat)}{lon_prefix}{abs(lon)}"


def build_download_link(tile: str, resolution=TileResolution.ONE_ARC_SECOND) -> str:
    base_link_fragment = "https://prd-tnm.s3.amazonaws.com/StagedProducts/Elevation/"
    resolution_link_fragment = f"{resolution.value}/TIFF/current/{tile}/"
    filename = f"USGS_{resolution.value}_{tile}.tif"
    link = base_link_fragment + resolution_link_fragment + filename

    return link


def download_tile(
    tile: str,
    output_dir: Path = Path("cache"),
    resolution=TileResolution.ONE_ARC_SECOND,
) -> Path:
    try:
        import requests
    except ImportError:
        raise ImportError(
            "requires requests to be installed. Try 'pip install requests'"
        )
    url = build_download_link(tile, resolution)
    filename = url.split("/")[-1]
    destination = output_dir / filename
    if destination.is_file():
        print(f"{str(destination)} already exists, skipping")
        return destination

    with requests.get(url, stream=True) as r:
        r.raise_for_status()

        destination.parent.mkdir(exist_ok=True)

        # write to file in chunks
        with destination.open("wb") as f:
            for chunk in r.iter_content(chunk_size=8192):
                f.write(chunk)

    return destination


def add_grade_to_graph(
    g,
    output_dir: Path = Path("cache"),
    resolution: TileResolution = TileResolution.ONE_ARC_SECOND,
):
    """
    Adds grade information to the edges of a graph.
    This will download the necessary elevation data from USGS as raster tiles and cache them in the output_dir.
    The resolution of the tiles can be specified with the resolution parameter.
    USGS has elevation data in increasing resolutions of: 1 arc-second and 1/3 arc-second
    Average tile file sizes for each resolution are about:

    * 1 arc-second: 50 MB
    * 1/3 arc-second: 350 MB

    Args:
        g (nx.MultiDiGraph): The networkx graph to add grades to.
        output_dir (Path, optional): The directory to cache the downloaded tiles in. Defaults to Path("cache").
        resolution (TileResolution, optional): The resolution of the tiles to download. Defaults to TileResolution.ONE_ARC_SECOND.

    Returns:
        nx.MultiDiGraph: The graph with grade information added to the edges.

    Example:
        >>> import osmnx as ox
        >>> g = ox.graph_from_place("Denver, Colorado, USA")
        >>> g = add_grade_to_graph(g)
    """
    try:
        import osmnx as ox
    except ImportError:
        raise ImportError("requires osmnx to be installed. Try 'pip install osmnx'")

    node_gdf = ox.graph_to_gdfs(g, nodes=True, edges=False)

    min_lat = node_gdf.y.min()
    max_lat = node_gdf.y.max()
    min_lon = node_gdf.x.min()
    max_lon = node_gdf.x.max()

    lats = cover_floats_with_integers(min_lat, max_lat)
    lons = cover_floats_with_integers(min_lon, max_lon)

    tiles = map(lat_lon_to_tile, itertools.product(lats, lons))

    files = []
    for tile in tiles:
        print(f"downloading tile {tile}")
        downloaded_file = download_tile(
            tile, output_dir=output_dir, resolution=resolution
        )
        files.append(str(downloaded_file))

    g = ox.add_node_elevations_raster(g, files)
    g = ox.add_edge_grades(g)

    return g
