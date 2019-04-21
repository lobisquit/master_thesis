socket_dir='data/aachen_net/postgres/socket_dir/'
# create and start local postgres session
mkdir -p data/aachen_net/postgres/
initdb -D data/aachen_net/postgres/

mkdir -p $(pwd)/$socket_dir
postgres -D data/aachen_net/postgres/ -k $(pwd)/$socket_dir &

dropdb nrw -h $(pwd)/$socket_dir
createdb nrw -h $(pwd)/$socket_dir
psql nrw -c 'CREATE EXTENSION postgis' -h $(pwd)/$socket_dir

echo "WARNING: this takes some time..."

shp2pgsql -s 4326 data/aachen_net/NRW_roads.shp roads | psql nrw -h $(pwd)/$socket_dir > /dev/null
shp2pgsql -s 4326 data/aachen_net/NRW_buildings.shp buildings | psql nrw -h $(pwd)/$socket_dir > /dev/null
