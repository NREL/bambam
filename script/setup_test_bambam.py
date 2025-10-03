print("importing osmnx, compass")
import osmnx as ox
from nrel.routee.compass.io import generate_compass_dataset


if __name__ == "__main__":
    print("downloading graph")
    g = ox.graph_from_place("Denver, Colorado, USA", network_type="drive")
    print("processing graph into compass dataset")
    generate_compass_dataset(g, output_directory="denver_co")

    # Boulder graph for GTFS
    g = ox.graph_from_place("Boulder, Colorado, USA", network_type="drive")
    print("processing graph into compass dataset")
    generate_compass_dataset(g, output_directory="boulder_co")