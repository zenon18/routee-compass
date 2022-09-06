import argparse
import getpass
import os
from pathlib import Path

from polestar.constructs.geometry import Geofence
from polestar.road_map import RoadMap
from sqlalchemy import create_engine

parser = argparse.ArgumentParser(description="Download a road map from Trolley")

parser.add_argument("geofence_file", help="Geofence file to use")
parser.add_argument("output_file", help="Output file to write to")


if __name__ == "__main__":
    args = parser.parse_args()
    user = os.environ.get("TROLLEY_USERNAME")
    if not user:
        user = input("Enter your trolley username: ")

    password = os.environ.get("TROLLEY_PASSWORD")
    if not password:
        password = getpass.getpass("Enter your trolley password: ")

    print("connecting to trolley..")
    engine = create_engine(
        f"postgresql://{user}:{password}@trolley.nrel.gov:5432/master"
    )

    print("loading geofence file..")
    geofence_path = Path(args.geofence_file)
    geofence = Geofence.from_geojson(geofence_path)

    print("building road map from sql..")
    rmap = RoadMap.from_sql(engine, geofence)

    print("writing road map to file..")
    outpath = Path(args.output_file)
    rmap.to_file(outpath)