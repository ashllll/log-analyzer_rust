/**
 * 固定容量的环形缓冲区
 * 当容量满时，新数据覆盖最旧的数据
 * 用于限制内存中保留的日志条目数量，防止内存泄漏
 */
export class CircularBuffer<T> {
  private buffer: T[];
  private head = 0;
  private size = 0;

  constructor(private readonly capacity: number) {
    this.buffer = new Array(capacity);
  }

  push(item: T): void {
    this.buffer[this.head] = item;
    this.head = (this.head + 1) % this.capacity;
    if (this.size < this.capacity) this.size++;
  }

  pushMany(items: T[]): void {
    for (const item of items) {
      this.push(item);
    }
  }

  toArray(): T[] {
    if (this.size === 0) return [];
    if (this.size < this.capacity) {
      return this.buffer.slice(0, this.size);
    }
    // 满了之后需要正确排序：从 head 到末尾 + 从 0 到 head
    return [
      ...this.buffer.slice(this.head),
      ...this.buffer.slice(0, this.head),
    ];
  }

  get length(): number {
    return this.size;
  }

  clear(): void {
    this.head = 0;
    this.size = 0;
    this.buffer = new Array(this.capacity);
  }
}
