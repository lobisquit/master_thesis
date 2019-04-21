socket_dir='data/aachen_net/postgres/socket_dir/'
# extract roads around aachen border
read aachen_border < data/aachen_net/aachen_border.txt

# due to noweb shortcomings, first newline and leading whitespaces have to be removed
query="
       SELECT osm_id, geom FROM roads
        WHERE fclass NOT IN ('trunk_link', 'bridleway', 'motorway',
                             'motorway_link', 'path', 'primary_link',
                             'secondary_link', 'service', 'steps',
                             'tertiary_link', 'track', 'track_grade2',
                             'track_grade3', 'track_grade4', 'track_grade5',
                             'unclassified', 'unknown')
          AND ST_Intersects(geom, ST_SetSRID(ST_GeomFromText('$aachen_border'), 4326));"
pgsql2shp -f data/aachen_net/aachen_roads -h $(pwd)/$socket_dir nrw \
          "$(echo ${query:1:${#query}} | sed 's/^[\t ]*//g')"

query="
       SELECT osm_id, geom, type FROM buildings
        WHERE ST_Intersects(geom, ST_SetSRID(ST_GeomFromText('$aachen_border'), 4326));"
pgsql2shp -f data/aachen_net/aachen_buildings -h $(pwd)/$socket_dir nrw \
          "$(echo ${query:1:${#query}} | sed 's/^[\t ]*//g')"
