import 'reflect-metadata';
import { createConnection } from 'typeorm';
import { SnakeNamingStrategy } from 'typeorm-naming-strategies';

import { Block } from './entity/Block';

createConnection({
  type: 'postgres',
  host: '10.1.1.20',
  port: 5432,
  username: 'koushiro',
  password: 'koushiro123',
  database: 'archive-dev',
  synchronize: false,
  logging: false,
  namingStrategy: new SnakeNamingStrategy(),
  entities: [Block],
})
  .then(async conn => {
    let [count, max] = await conn
      .getRepository(Block)
      .createQueryBuilder()
      .select('COUNT(*), MAX(block_num)')
      .getRawOne()
      .then(({ count, max }) => [+count, +max]);
    let min = max - count + 1;
    console.log(`Block #${min} ~ Block #${max}: Count(${count})`);

    let counter = 0;
    for (let i = max; i > min + 1; i--) {
      let [block, parentBlock] = await Promise.all([
        conn.getRepository(Block).findOne({ blockNum: i }),
        conn.getRepository(Block).findOne({ blockNum: i - 1 }),
      ]);
      if (
        block &&
        parentBlock &&
        block.parentHash.toString('hex') !== parentBlock.blockHash.toString('hex')
      ) {
        console.log(`Block #${i}, parentHash 0x${block.parentHash.toString('hex')}`);
        console.log(`ParentBlock #${i - 1}, blockHash 0x${parentBlock.blockHash.toString('hex')}`);
      }
      counter += 1;
      process.stdout.write(`(${counter} / ${count})\r`);
    }
  })
  .catch(error => console.log(error));
