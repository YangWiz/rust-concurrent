# Promising Semantics
[slides](https://docs.google.com/presentation/d/1NMg08N1LUNDPuMxNZ-UMbdH13p8LXgMM3esbWRMowhU/edit#slide=id.g626c37cc1d_110_0)

### 一种用来描述 松散的行为和顺序的 交错操作型语义模型
### Interleaving operational semantics modeling relaxed	behaviors and orderings.
* 所有的语句都是顺序执行，用模型来对 1. CPU乱序执行 2. 编译器优化 来建模
1. 对 load hoisting 建模，w/ 多值内存（multi-valued memory）
* 允许一个线程对一个位置的旧值进行访问
* Allowing a thread to read an old value from a location.
2. 对 read-modify-write 建模，w/ 临接信息 （message adjacency）
* 禁止对同一个值进行同时的读写操作 (原子读写)
* Forbidding multiple read-modify-write operations of a single value.
3. 对 相关性 (coherence) & 顺序 (ordering) 建模 w/ 视界 (views)
* 限制一个线程的行为
* constraining a thread's behavior.
4. 对 store hoisting 进行建模 w/ promise
* 允许一个线程激进地写入一个值
* Allowing a thread speculatively write a value.

#### Load Hoisting:  
r1 == r2 == 0 is possible.  
all the threads don't have data dependencies.
```
X = 1       ||        Y = 1
r1 = Y      ||        r2 = X
```

#### Read-modify-write:  
r1 == r2 == 0 is impossible.  
risc-v: csr operation r/w (atomically)
```
X.fetch_and_add()	|| 	  Y.fetch_and_add()
```

# views:
constrain a thread's behaviors.  
multi-value allows so many unintended behaviors.  
* Constrain!  

View: Location -> Timestamp (acknowledging messages for each location)  
对每个位置变化的应答 
* Per-thread view for cohenrence.
* 每个线程的视界对应确保相关性
* Per-message view for release/acquire synchronization
* 每个信息的视界对应释放和获取的同步
* A global view for SC (sequentially consistency) synchronization.
* 全局视界对应内存屏障同步

#### Per-thread view (single thread is executed sequentially for the same location):

Reading/Writing happens after the current thread's view.
Reading/Writing changes the current thread's view.
```
RR 			X1 = 1 || r1 = X1; r2 = X1 (if r1 == 1, r2 must be 1) 
RW 			r1 = X1; X1 = 2 (r1 is 0, and X1 is 2)
WR			X1 = 1; r1 = X1 (r1 = 1)
WW			X1 = 1; X1 = 2 (X1 = 2)
```

#### Per-message view (modeling release and acquire)

store(release) => load(acquire)
release operation (1. all the value before the release will be visible for other threads. 2. all the value before the the release can not pass after the release operation.) will create a boundary for the acquire operation. 

#### A global view representing the accumulated view of SC fences (similar to per-message view)

After an SC fence, SC view and thread’s view become the maximum of them.
SC fence updates the SC view state after calling SC fence api.

* 全局视界和消息视界的区别：全局视界在每次调用内存屏障接口后，视界更新至最新状态。而消息视界只在释放时更新，并且只有获取操作才能保证得到最新的状态。