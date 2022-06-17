/// Example showing usage of test_any(), test_some() and test_all().
use mpi;
use mpi::Rank;
use mpi::traits::*;
use mpi::request::{Scope, RequestCollection};

const COUNT: usize = 128;

/// Send and receive COUNT number of immediate requests.
fn send_recv<'a, C: Communicator, S: Scope<'a> + Copy>(
    world: C,
    scope: S,
    coll: &mut RequestCollection<'a, [i32; 4]>,
    next_proc: Rank,
    prev_proc: Rank,
    x: &'a [[i32; 4]],
    recv: &'a mut [[i32; 4]],
) {
    for elm in x {
        let sreq = world
            .process_at_rank(next_proc)
            .immediate_send(scope, elm);
        coll.add(sreq);
    }
    for elm in recv.iter_mut() {
        let rreq = world
            .process_at_rank(prev_proc)
            .immediate_receive_into(scope, elm);
        coll.add(rreq);
    }
}

/// Ensure that the result buffer, containing the data for both send and receive
/// requests, matches what was sent in the buffer x.
fn check_result_buffer(x: &[[i32; 4]], mut result_buf: Vec<[i32; 4]>) {
    result_buf.sort();
    assert_eq!(result_buf.len(), 2 * x.len());
    for (i, x_val) in x.iter().enumerate() {
        // The result buffer has both send values and receive values
        assert_eq!(result_buf[2 * i], *x_val);
        assert_eq!(result_buf[2 * i + 1], *x_val);
    }
}

fn main() {
    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    let rank = world.rank();
    let size = world.size();

    let next_proc = (rank + 1) % size;
    let prev_proc = if rank > 0 { rank - 1 } else { size - 1 };

    let x: Vec<[i32; 4]> = (0..COUNT as i32).map(|i| [i, i + 1, i + 2, i + 3]).collect();
    let mut recv: Vec<[i32; 4]> = vec![[0, 0, 0, 0]; COUNT];
    mpi::request::multiple_scope(COUNT, |scope, coll| {
        send_recv(world, scope, coll, next_proc, prev_proc, &x, &mut recv);

        let mut buf = vec![];
        while coll.incomplete() > 0 {
            if let Some((_, _, data)) = coll.test_any() {
                buf.push(*data);
            }
        }
        check_result_buffer(&x, buf);
    });

    let mut recv: Vec<[i32; 4]> = vec![[0, 0, 0, 0]; COUNT];
    mpi::request::multiple_scope(COUNT, |scope, coll| {
        send_recv(world, scope, coll, next_proc, prev_proc, &x, &mut recv);

        let mut complete = vec![None; 2 * COUNT];
        let mut buf = vec![];
        while coll.incomplete() > 0 {
            let count = coll.test_some(&mut complete);
            if count > 0 {
                println!("test_some(): {} request(s) completed", count);
                for elm in complete.iter() {
                    if let Some((_, _, data)) = elm {
                        buf.push(**data);
                    }
                }
            }
        }
        check_result_buffer(&x, buf);
    });

    let mut recv: Vec<[i32; 4]> = vec![[0, 0, 0, 0]; COUNT];
    mpi::request::multiple_scope(COUNT, |scope, coll| {
        send_recv(world, scope, coll, next_proc, prev_proc, &x, &mut recv);

        let mut complete = vec![None; 2 * COUNT];
        while !coll.test_all(&mut complete) { }
        let buf: Vec<[i32; 4]> = complete.iter().map(|elm| *elm.unwrap().2).collect();
        check_result_buffer(&x, buf);
    });
}
