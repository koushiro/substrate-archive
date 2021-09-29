import { Entity, PrimaryColumn, Column } from 'typeorm';

@Entity({ synchronize: false })
export class Block {
  @PrimaryColumn({ type: 'integer' })
  blockNum!: number;
  @Column({ type: 'bytea' })
  blockHash!: Buffer;
  @Column({ type: 'bytea' })
  parentHash!: Buffer;
}
