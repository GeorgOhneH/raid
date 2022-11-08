use crate::matrix::Matrix;
use std::path::PathBuf;

use crate::galois;
use crate::galois::Galois;
use crate::raid::RAID;
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use std::convert::TryInto;
use std::fs;
use std::fs::create_dir;
use std::thread::JoinHandle;

#[derive(Debug)]
pub enum HeadNodeMsg<const X: usize> {
    Data {
        data_slice: usize,
        data: [Galois; X],
        data_idx: usize,
    },
}

#[derive(Debug)]
pub enum Msg<const X: usize> {
    NewData {
        data_slice: usize,
        data: [Galois; X],
    },
    NewDataChecksum {
        data_slice: usize,
        data: [Galois; X],
        dev_idx: usize,
    },
    UpdateData {
        data_slice: usize,
        data: [Galois; X],
    },
    UpdateDataChecksum {
        data_slice: usize,
        diff: [Galois; X],
        dev_idx: usize,
    },
    NeedRecover {
        data_slice: usize,
        dev_idx: usize,
    },
    HeadNodeDataRequest {
        data_slice: usize,
    },
    DestroyStorage,
}

#[derive(Debug)]
pub enum RecoverMsg<const X: usize> {
    RequestedData {
        data_slice: usize,
        data: [Galois; X],
        dev_idx: usize,
    },
}

pub struct Node<const D: usize, const C: usize, const X: usize>
where
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    dev_idx: usize,
    vandermonde: Matrix<C, D>,
    path: PathBuf,
    coms: [Sender<Msg<X>>; D + C],
    recover_coms: [Sender<RecoverMsg<X>>; D + C],
    data_slices: usize,
    head_node: Sender<HeadNodeMsg<X>>,
}

impl<const D: usize, const C: usize, const X: usize> Node<D, C, X>
where
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    pub fn new(
        path: PathBuf,
        dev_idx: usize,
        vandermonde: Matrix<C, D>,
        coms: [Sender<Msg<X>>; D + C],
        recover_coms: [Sender<RecoverMsg<X>>; D + C],
        head_node: Sender<HeadNodeMsg<X>>,
    ) -> Self {
        let _ = std::fs::remove_dir_all(&path);
        create_dir(&path).unwrap();
        Self {
            path,
            dev_idx,
            vandermonde,
            coms,
            recover_coms,
            head_node,
            data_slices: 0,
        }
    }

    fn data_check_idx(dev_idx: usize, data_slice: usize) -> usize {
        ((dev_idx as isize - data_slice as isize).rem_euclid((D + C) as isize)) as usize
    }

    fn data_idx(&self, data_slice: usize) -> usize {
        let idx = Self::data_check_idx(self.dev_idx, data_slice);
        if idx >= D {
            panic!("not good");
        }
        idx
    }

    fn check_idx(&self, data_slice: usize) -> usize {
        let idx = Self::data_check_idx(self.dev_idx, data_slice);
        if idx < D || idx >= C + D {
            panic!("not good {} {} {}", idx, self.dev_idx, data_slice);
        }
        idx - D
    }

    fn data_name(&self, data_slice: usize) -> String {
        let idx = self.data_idx(data_slice);
        format!("{}_{}d.bin", data_slice, idx)
    }

    fn checksum_name(&self, data_slice: usize) -> String {
        let idx = self.check_idx(data_slice);
        format!("{}_{}c.bin", data_slice, idx)
    }

    fn data_file(&self, data_slice: usize) -> PathBuf {
        let name = self.data_name(data_slice);
        self.path.join(name)
    }
    fn checksum_file(&self, data_slice: usize) -> PathBuf {
        let name = self.checksum_name(data_slice);
        self.path.join(name)
    }

    fn read_data(&self, data_slice: usize) -> [Galois; X] {
        let file_path = self.data_file(data_slice);
        galois::from_bytes(fs::read(file_path).unwrap().try_into().unwrap())
    }

    fn read_checksum(&self, data_slice: usize) -> [Galois; X] {
        let file_path = self.checksum_file(data_slice);
        galois::from_bytes(fs::read(file_path).unwrap().try_into().unwrap())
    }

    fn write_data(&self, data_slice: usize, data: &[Galois; X]) {
        let file_path = self.data_file(data_slice);
        fs::write(file_path, galois::as_bytes_ref(data)).unwrap();
    }

    fn write_checksum(&self, data_slice: usize, check: &[Galois; X]) {
        let file_path = self.checksum_file(data_slice);
        fs::write(file_path, galois::as_bytes_ref(check)).unwrap();
    }

    pub fn start(mut self, rec: Receiver<Msg<X>>, recover_rec: Receiver<RecoverMsg<X>>) {
        while let Ok(msg) = rec.recv() {
            //println!("thread{}: {:?}", self.dev_idx, &msg);
            match msg {
                Msg::NewData { data_slice, data } => {
                    for check_idx in 0..C {
                        let check_dev = HeadNode::<D, C, X>::dev_idx(data_slice, check_idx + D);
                        self.coms[check_dev]
                            .send(Msg::NewDataChecksum {
                                data_slice,
                                data: data.clone(),
                                dev_idx: self.dev_idx,
                            })
                            .unwrap()
                    }
                    self.data_slices = self.data_slices.max(data_slice);
                    self.write_data(data_slice, &data);
                }
                Msg::UpdateData { data_slice, data } => {
                    let old_data = self.read_data(data_slice);
                    let diff_data = core::array::from_fn(|i| data[i] - old_data[i]);
                    for check_idx in 0..C {
                        let check_dev = HeadNode::<D, C, X>::dev_idx(data_slice, check_idx + D);
                        self.coms[check_dev]
                            .send(Msg::UpdateDataChecksum {
                                data_slice,
                                diff: diff_data.clone(),
                                dev_idx: self.dev_idx,
                            })
                            .unwrap()
                    }
                    self.write_data(data_slice, &data);
                }
                Msg::NewDataChecksum {
                    data_slice,
                    data,
                    dev_idx,
                } => {
                    self.data_slices = self.data_slices.max(data_slice);
                    let data_idx = Self::data_check_idx(dev_idx, data_slice);
                    let file_path = self.checksum_file(data_slice);
                    let current_checksum = if file_path.exists() {
                        self.read_checksum(data_slice)
                    } else {
                        [Galois::zero(); X]
                    };
                    let new_checksum = core::array::from_fn(|i| {
                        current_checksum[i]
                            + self.vandermonde[self.check_idx(data_slice)][data_idx] * data[i]
                    });
                    self.write_checksum(data_slice, &new_checksum);
                }
                Msg::UpdateDataChecksum {
                    data_slice,
                    diff,
                    dev_idx,
                } => {
                    let data_idx = Self::data_check_idx(dev_idx, data_slice);
                    let current_checksum = self.read_checksum(data_slice);
                    let new_checksum = core::array::from_fn(|i| {
                        current_checksum[i]
                            + self.vandermonde[self.check_idx(data_slice)][data_idx] * diff[i]
                    });
                    self.write_checksum(data_slice, &new_checksum);
                }
                Msg::DestroyStorage => {
                    let _ = std::fs::remove_dir_all(&self.path);
                    create_dir(&self.path).unwrap();
                    self.recover(&recover_rec)
                }
                Msg::NeedRecover {
                    data_slice,
                    dev_idx,
                } => {
                    let data = if Self::data_check_idx(self.dev_idx, data_slice) < D {
                        self.read_data(data_slice)
                    } else {
                        self.read_checksum(data_slice)
                    };
                    self.recover_coms[dev_idx]
                        .send(RecoverMsg::RequestedData {
                            data_slice,
                            data,
                            dev_idx: self.dev_idx,
                        })
                        .unwrap();
                }
                Msg::HeadNodeDataRequest { data_slice } => {
                    let data = self.read_data(data_slice);
                    self.head_node
                        .send(HeadNodeMsg::Data {
                            data_slice,
                            data,
                            data_idx: self.data_idx(data_slice),
                        })
                        .unwrap();
                }
            }
        }
    }

    pub fn recover(&self, recover_rec: &Receiver<RecoverMsg<X>>) {
        for current_data_slice in 0..self.data_slices + 1 {
            while !recover_rec.is_empty() {
                recover_rec.recv().unwrap();
            }
            for i in 0..C + D {
                if i != self.dev_idx {
                    self.coms[i]
                        .send(Msg::NeedRecover {
                            dev_idx: self.dev_idx,
                            data_slice: current_data_slice,
                        })
                        .unwrap();
                }
            }
            let mut r_data = vec![];
            let mut r_check = vec![];
            let mut r_data_idx = vec![];
            let mut r_check_idx = vec![];
            while let Ok(msg) = recover_rec.recv() {
                //println!("thread{}: msg {:?}", self.dev_idx, &msg);
                match msg {
                    RecoverMsg::RequestedData {
                        data_slice,
                        data,
                        dev_idx,
                    } => {
                        if data_slice != current_data_slice {
                            continue;
                        }
                        let data_check_idx = Self::data_check_idx(dev_idx, data_slice);
                        if data_check_idx < D {
                            r_data.push(data);
                            r_data_idx.push(data_check_idx);
                        } else {
                            r_check.push(data);
                            r_check_idx.push(data_check_idx - D)
                        }
                        if r_data_idx.len() + r_check_idx.len() == D {
                            break;
                        }
                    }
                }
            }

            r_data.append(&mut r_check);
            let mut rec_matrix = self.vandermonde.recovery_matrix(r_data_idx, r_check_idx);
            let rec_data = rec_matrix.gaussian_elimination(r_data.try_into().unwrap());

            let data_check_idx = Self::data_check_idx(self.dev_idx, current_data_slice);
            if data_check_idx < D {
                self.write_data(current_data_slice, &rec_data[data_check_idx])
            } else {
                let checksum = self.vandermonde.mul_vec_at(&rec_data, data_check_idx - D);
                self.write_checksum(current_data_slice, &checksum);
            }
        }
    }
}

pub struct HeadNode<const D: usize, const C: usize, const X: usize>
where
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    data_slices: usize,
    handles: [JoinHandle<()>; D + C],
    coms: [Sender<Msg<X>>; D + C],
    receiver: Receiver<HeadNodeMsg<X>>,
}

impl<const D: usize, const C: usize, const X: usize> HeadNode<D, C, X>
where
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    fn dev_idx(data_slice: usize, data_idx: usize) -> usize {
        (data_idx + data_slice) % (D + C)
    }
}

impl<const D: usize, const C: usize, const X: usize> RAID<D, C, X> for HeadNode<D, C, X>
where
    [(); C + D]:,
    [(); D + C]:,
    [(); C + C]:,
    [(); D + D]:,
{
    fn new(root_path: PathBuf) -> Self {
        let paths: [PathBuf; D + C] =
            core::array::from_fn(|i| root_path.join(format!("device{i}")));
        for path in &paths {
            let _ = std::fs::remove_dir_all(path);
            create_dir(path).unwrap()
        }

        let channels: [(Sender<Msg<X>>, Receiver<Msg<X>>); D + C] =
            core::array::from_fn(|_| unbounded());
        let recover_channels: [(Sender<RecoverMsg<X>>, Receiver<RecoverMsg<X>>); D + C] =
            core::array::from_fn(|_| unbounded());

        let (h_send, h_rec) = unbounded();

        let coms = core::array::from_fn(|i| channels[i].0.clone());
        let recover_coms = core::array::from_fn(|i| recover_channels[i].0.clone());

        let vandermonde = Matrix::<C, D>::reed_solomon();

        let handles = core::array::from_fn(|i| {
            let path = paths[i].clone();
            let v = vandermonde.clone();
            let c = coms.clone();
            let rec_c = recover_coms.clone();
            let r = channels[i].1.clone();
            let rec_r = recover_channels[i].1.clone();
            let h_send_clone = h_send.clone();
            std::thread::Builder::new()
                .name(format!("thread{i}"))
                .spawn(move || {
                    let node = Node::new(path, i, v, c, rec_c, h_send_clone);
                    node.start(r, rec_r)
                })
                .unwrap()
        });

        Self {
            data_slices: 0,
            handles,
            coms,
            receiver: h_rec,
        }
    }

    fn add_data(&mut self, data: &[[u8; X]; D]) -> usize {
        let data: &[[Galois; X]; D] = unsafe { core::mem::transmute(data) };
        for data_idx in 0..D {
            let dev_idx = Self::dev_idx(self.data_slices, data_idx);
            self.coms[dev_idx]
                .send(Msg::NewData {
                    data_slice: self.data_slices,
                    data: data[data_idx].clone(),
                })
                .unwrap()
        }

        self.data_slices += 1;
        self.data_slices - 1
    }

    fn read_data(&self, data_slice: usize) -> [[u8; X]; D] {
        for data_idx in 0..D {
            let dev_idx = Self::dev_idx(data_slice, data_idx);
            self.coms[dev_idx]
                .send(Msg::HeadNodeDataRequest { data_slice })
                .unwrap()
        }

        let mut count = 0;
        let mut result = [[0u8; X]; D];
        while count < D {
            let msg = self.receiver.recv().unwrap();
            match msg {
                HeadNodeMsg::Data {
                    data_slice,
                    data,
                    data_idx,
                } => result[data_idx] = galois::as_bytes(data),
            }
            count += 1;
        }

        result
    }

    fn destroy_devices(&self, dev_idxs: &[usize]) {
        for dev_idx in dev_idxs {
            self.coms[*dev_idx].send(Msg::DestroyStorage).unwrap()
        }
    }

    fn update_data(&self, data: &[u8; X], data_slice: usize, data_idx: usize) {
        let data = galois::from_bytes_ref(data).clone();
        let dev_idx = Self::dev_idx(data_slice, data_idx);
        self.coms[dev_idx]
            .send(Msg::UpdateData { data_slice, data })
            .unwrap()
    }
}
