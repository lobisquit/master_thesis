population_density='http://offenedaten.aachen.de/dataset/81650028-ef21-4f1b-a991-9e3a3f01c729/resource/460bfe18-7df4-49fb-b5d0-6dfc1d0cffd5/download/20170630opendataaachen-daten-statistische-bezirkealle.csv
'
district_map='http://offenedaten.aachen.de/dataset/5ea893af-8f1d-4658-9066-8f05daed1022/resource/6dfc1b81-26d9-4ed8-b8c4-a61013659f51/download/statistischebezirkeaachen.zip
'
NRW_map='http://download.geofabrik.de/europe/germany/nordrhein-westfalen-latest-free.shp.zip
'
mkdir -p data/aachen_net/

# download
wget -c $population_density -O data/aachen_net/20170630_population_density_temp.csv
wget -c $district_map -O data/aachen_net/district_map.zip
wget -c $NRW_map -O data/aachen_net/NRW_map.zip

# preprocess
awk -F, '{print $1 "," $3}' data/aachen_net/20170630_population_density_temp.csv > data/aachen_net/20170630_population_density.csv
rm -f data/aachen_net/20170630_population_density_temp.csv

# gather city district borders
unzip -p data/aachen_net/district_map.zip StatistischeBezirkeAachen.shp > data/aachen_net/aachen_district_map.shp
unzip -p data/aachen_net/district_map.zip StatistischeBezirkeAachen.shx > data/aachen_net/aachen_district_map.shx
unzip -p data/aachen_net/district_map.zip StatistischeBezirkeAachen.dbf > data/aachen_net/aachen_district_map.dbf
unzip -p data/aachen_net/district_map.zip StatistischeBezirkeAachen.prj > data/aachen_net/aachen_district_map.prj

# gather NRW roads
unzip -p data/aachen_net/NRW_map.zip gis_osm_roads_free_1.shp > data/aachen_net/NRW_roads.shp
unzip -p data/aachen_net/NRW_map.zip gis_osm_roads_free_1.shx > data/aachen_net/NRW_roads.shx
unzip -p data/aachen_net/NRW_map.zip gis_osm_roads_free_1.dbf > data/aachen_net/NRW_roads.dbf

# gather NRW buildings
unzip -p data/aachen_net/NRW_map.zip gis_osm_buildings_a_free_1.shp > data/aachen_net/NRW_buildings.shp
unzip -p data/aachen_net/NRW_map.zip gis_osm_buildings_a_free_1.shx > data/aachen_net/NRW_buildings.shx
unzip -p data/aachen_net/NRW_map.zip gis_osm_buildings_a_free_1.dbf > data/aachen_net/NRW_buildings.dbf
