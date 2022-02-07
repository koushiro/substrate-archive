# Substrate Archive

This is my fork version of [substrate-archive](https://github.com/paritytech/substrate-archive)

## Requirement

 - Substrate Node (RocksDB)
 - PostgreSQL 12+
 - Kafka 2.13+ (Optional)

## Components

 - `archive-client`: A specialized substrate client for `Archive`.
 - `archive-postgres`: PostgreSQL related operations for `Archive`.
 - `archive-kafka`: Kafka related operations for `Archive`.
 - `archive-actor`: Specified logic of each `Archive` component, based on actor model.
   - scheduler: The scheduler used to get the blocks.
     - block: Get the specified block with storage changes.
     - best_and_finalized: Get the best block (number + hash) and finalized block (number + hash).
   - metadata: Get the metadata (spec version) of the blocks.
   - database(postgres): Store metadata and block (with storage changes) message into database.
   - dispatcher: Dispatch metadata and block message (with storage changes) to other targets.
     - kafka: Publish the metadata and block message (with storage changes) to kafka.
 - `archive-primitives`: Runtime primitives.

## Architecture

```

                 +------------------------------------------------+                              
                 |                                                |                              
                 |          +---------+                           |                     +-------+
                 |   +------+  block  +-------+                   |                  +--+ kafka |
                 |   |      +---------+       |                   |                  |  +-------+
                 |   |                        |                   |                  |           
 +-----------+   |   |                        |   +------------+  |  +------------+  |           
 |           |   |   |      +---------+       |   |            |  |  |            |  |  +-------+
 |  backend  +-------+------+  block  +-------+---+  metadata  +--+--+  database  +--+--+  ...  |
 |           |   |   |      +---------+       |   |            |  |  |            |  |  +-------+
 +-----+-----+   |   |                        |   +------+-----+  |  +------------+  |           
       |         |   |                        |          |        |                  |           
       |         |   |      +---------+       |          |        |                  |  +-------+
       |         |   +------+   ...   +-------+     +----+----+   |                  +--+  ...  |
 +-----+-----+   |          +---------+             | genesis |   |                     +-------+
 |  RocksDB  |   |                                  +---------+   |                              
 +-----------+   | scheduler                                      |                              
                 +------------------------------------------------+                              

```

## License

Under the GPL-v3 license. See the [LICENSE](LICENSE) file for details.
