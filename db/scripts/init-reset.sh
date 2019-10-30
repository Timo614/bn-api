set -e
set -o allexport
source .env
set +o allexport
# Run this script to update the database to the latest migration version
diesel -V 2> /dev/null || cargo install diesel_cli --no-default-features --features postgres
# Since we are not using bndb_cli to run the migrations, we need to run the functions.sql
# Create a tmp migration dir as the last migration.
MIGRATION_FUNCTIONS_DIR="migrations/$(date +%Y%m%d%H%M%S)_internal_functions"
# Link the functions.sql file as the up.sql
mkdir "$MIGRATION_FUNCTIONS_DIR" &&  ln -s "$(pwd)/functions/functions.sql" "$MIGRATION_FUNCTIONS_DIR/up.sql" && touch "$MIGRATION_FUNCTIONS_DIR/down.sql"

diesel database reset --database-url=$TEST_DATABASE_ADMIN_URL
diesel setup --database-url=$TEST_DATABASE_ADMIN_URL
diesel migration run --database-url=$TEST_DATABASE_ADMIN_URL

# Run this script to update the database to the latest migration version
diesel -V 2> /dev/null || cargo install diesel_cli --no-default-features --features postgres
diesel database reset --database-url=$DATABASE_ADMIN_URL
diesel setup --database-url=$DATABASE_ADMIN_URL
diesel migration run --database-url=$DATABASE_ADMIN_URL

# Delete the tmp migration folder
rm -rf "$MIGRATION_FUNCTIONS_DIR"

if [[ "$1" == "--seed" ]]; then
   cargo run seed -c $DATABASE_URL
fi
