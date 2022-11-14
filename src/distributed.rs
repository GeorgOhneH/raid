use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::fs::create_dir;
use std::path::PathBuf;
use std::io;
use std::thread::JoinHandle;

use crossbeam_channel::{unbounded, Receiver, Sender, SendError};

use crate::galois;
use crate::galois::Galois;
use crate::matrix::Matrix;
use crate::raid::RAID;


#[derive(Debug)]
pub enum Error {
    Shutdown
}

impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self { Self::Shutdown }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct HeadNodeMsg<const X: usize> {
    data_slice: usize,
    data: Box<[Galois; X]>,
}

#[derive(Debug)]
pub enum Msg<const X: usize> {
    NewData {
        data_slice: usize,
        data: Box<[Galois; X]>,
    },
    NewDataAt {
        data_slice: usize,
        data: Box<[Galois; X]>,
    },
    NewDataChecksum {
        data_slice: usize,
        data: Box<[Galois; X]>,
        dev_idx: usize,
    },
    NewDataChecksumAt {
        data_slice: usize,
        data: Box<[Galois; X]>,
        dev_idx: usize,
    },
    UpdateData {
        data_slice: usize,
        data: Box<[Galois; X]>,
    },
    UpdateDataChecksum {
        data_slice: usize,
        diff: Box<[Galois; X]>,
        dev_idx: usize,
    },
    NeedRecover {
        data_slice: usize,
        dev_idx: usize,
    },
    HeadNodeDataRequest {
        data_slice: usize,
        oneshot_send: oneshot::Sender<HeadNodeMsg<X>>,
    },
    DestroyStorage {
        max_data_slice: usize,
        oneshot_send: oneshot::Sender<()>,
    },
    Shutdown,
}

#[derive(Debug)]
pub enum RecoverMsg<const X: usize> {
    RequestedData {
        data_slice: usize,
        data: Box<[Galois; X]>,
        dev_idx: usize,
    },
}

struct CurrentChecksumStatus<const X: usize> {
    count: usize,
    current_checksum: Box<[Galois; X]>,
    missed_recover_dev_idx: Vec<usize>,
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
    current_checksum: HashMap<usize, CurrentChecksumStatus<X>>,
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
    ) -> Self {
        let _ = std::fs::remove_dir_all(&path);
        create_dir(&path).unwrap();
        Self {
            path,
            dev_idx,
            vandermonde,
            coms,
            recover_coms,
            current_checksum: HashMap::new(),
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

    fn read_data(&self, data_slice: usize) -> Box<[Galois; X]> {
        let file_path = self.data_file(data_slice);
        match fs::read(&file_path) {
            Ok(file) => {
                galois::from_bytes(file.into_boxed_slice().try_into().unwrap())
            }
            Err(err) => {
                let io::ErrorKind::NotFound = err.kind() else {
                    panic!("{:?}", err)
                };
                galois::zeros()
            }
        }
    }

    fn read_checksum(&self, data_slice: usize) -> Box<[Galois; X]> {
        let file_path = self.checksum_file(data_slice);
        galois::from_bytes(fs::read(file_path).unwrap().into_boxed_slice().try_into().unwrap())
    }

    fn write_data(&self, data_slice: usize, data: &[Galois; X]) {
        let file_path = self.data_file(data_slice);
        fs::write(file_path, galois::as_bytes_ref(data)).unwrap();
    }

    fn write_checksum(&self, data_slice: usize, check: &[Galois; X]) {
        let file_path = self.checksum_file(data_slice);
        fs::write(file_path, galois::as_bytes_ref(check)).unwrap();
    }

    pub fn start(mut self, rec: Receiver<Msg<X>>, recover_rec: Receiver<RecoverMsg<X>>) -> Result<()> {
        while let Ok(msg) = rec.recv() {
            match msg {
                Msg::NewData { data_slice, data } => {
                    for check_idx in 0..C {
                        let check_dev = HeadNode::<D, C, X>::dev_idx(data_slice, check_idx + D);
                        self.coms[check_dev]
                            .send(Msg::NewDataChecksum {
                                data_slice,
                                data: data.clone(),
                                dev_idx: self.dev_idx,
                            })?;
                    }
                    self.write_data(data_slice, &data);
                }
                Msg::NewDataAt { data_slice, data } => {
                    for check_idx in 0..C {
                        let check_dev = HeadNode::<D, C, X>::dev_idx(data_slice, check_idx + D);
                        self.coms[check_dev]
                            .send(Msg::NewDataChecksumAt {
                                data_slice,
                                data: data.clone(),
                                dev_idx: self.dev_idx,
                            })?;
                    }
                    self.write_data(data_slice, &data);
                }
                Msg::UpdateData { data_slice, data } => {
                    let old_data = self.read_data(data_slice);
                    let diff_data = galois::from_fn(|i| data[i] - old_data[i]);
                    for check_idx in 0..C {
                        let check_dev = HeadNode::<D, C, X>::dev_idx(data_slice, check_idx + D);
                        self.coms[check_dev]
                            .send(Msg::UpdateDataChecksum {
                                data_slice,
                                diff: diff_data.clone(),
                                dev_idx: self.dev_idx,
                            })?;
                    }
                    self.write_data(data_slice, &data);
                }
                Msg::NewDataChecksum {
                    data_slice,
                    data,
                    dev_idx,
                } => {
                    let data_idx = Self::data_check_idx(dev_idx, data_slice);

                    let current_status = self.current_checksum.get(&data_slice);

                    let zero = galois::zeros::<X>();
                    let new_status = if let Some(status) = current_status {
                        let new_checksum = galois::from_fn(|i| {
                            status.current_checksum[i]
                                + self.vandermonde[self.check_idx(data_slice)][data_idx] * data[i]
                        });
                        CurrentChecksumStatus {
                            count: status.count + 1,
                            current_checksum: new_checksum,
                            missed_recover_dev_idx: status.missed_recover_dev_idx.clone(),
                        }
                    } else {
                        let new_checksum = galois::from_fn(|i| {
                            self.vandermonde[self.check_idx(data_slice)][data_idx] * data[i]
                        });
                        CurrentChecksumStatus {
                            count: 1,
                            current_checksum: new_checksum,
                            missed_recover_dev_idx: vec![],
                        }
                    };
                    if new_status.count == D {
                        self.current_checksum.remove(&data_slice);
                        self.write_checksum(data_slice, &new_status.current_checksum);
                        for dev_idx in new_status.missed_recover_dev_idx {
                            self.recover_coms[dev_idx]
                                .send(RecoverMsg::RequestedData {
                                    data_slice,
                                    data: new_status.current_checksum.clone(),
                                    dev_idx: self.dev_idx,
                                })?;
                        }
                    } else {
                        self.current_checksum.insert(data_slice, new_status);
                    }
                }
                Msg::UpdateDataChecksum {
                    data_slice,
                    diff,
                    dev_idx,
                } => {
                    let data_idx = Self::data_check_idx(dev_idx, data_slice);
                    let self_check_idx = self.check_idx(data_slice);
                    let current_status = self.current_checksum.get_mut(&data_slice);
                    if let Some(current_status) = current_status {
                        let new_checksum = galois::from_fn(|i| {
                            current_status.current_checksum[i]
                                + self.vandermonde[self_check_idx][data_idx] * diff[i]
                        });
                        current_status.current_checksum = new_checksum;
                    } else {
                        let current_checksum = self.read_checksum(data_slice);
                        let new_checksum = galois::from_fn(|i| {
                            current_checksum[i]
                                + self.vandermonde[self.check_idx(data_slice)][data_idx] * diff[i]
                        });
                        self.write_checksum(data_slice, &new_checksum);
                    }
                }
                Msg::NewDataChecksumAt {
                    data_slice,
                    data,
                    dev_idx,
                } => {
                    assert!(self.current_checksum.get(&data_slice).is_none());
                    let data_idx = Self::data_check_idx(dev_idx, data_slice);
                    let self_check_idx = self.check_idx(data_slice);
                    let checksum_path = self.checksum_file(data_slice);
                    let new_checksum: Box<[Galois; X]> = match fs::read(&checksum_path) {
                        Ok(file) => {
                            let old_checksum: Box<[Galois; X]> = galois::from_bytes(file.into_boxed_slice().try_into().unwrap());
                            galois::from_fn(|i| {
                                old_checksum[i] + self.vandermonde[self_check_idx][data_idx] * data[i]
                            })
                        }
                        Err(err) => {
                            let io::ErrorKind::NotFound = err.kind() else {
                                panic!("{:?}", err)
                            };
                            galois::from_fn(|i| {
                                self.vandermonde[self_check_idx][data_idx] * data[i]
                            })
                        }
                    };
                    fs::write(&checksum_path, galois::as_bytes_ref(&new_checksum)).unwrap();
                }
                Msg::DestroyStorage {
                    max_data_slice, oneshot_send,
                } => {
                    let _ = std::fs::remove_dir_all(&self.path);
                    create_dir(&self.path).unwrap();
                    self.recover(&recover_rec, max_data_slice)?;
                    oneshot_send.send(()).unwrap();
                }
                Msg::NeedRecover {
                    data_slice,
                    dev_idx,
                } => {
                    if Self::data_check_idx(self.dev_idx, data_slice) < D {
                        self.recover_coms[dev_idx]
                            .send(RecoverMsg::RequestedData {
                                data_slice,
                                data: self.read_data(data_slice),
                                dev_idx: self.dev_idx,
                            })
                            .unwrap();
                    } else if let Some(checksum_status) = self.current_checksum.get_mut(&data_slice) {
                        checksum_status.missed_recover_dev_idx.push(dev_idx);
                    } else {
                        self.recover_coms[dev_idx]
                            .send(RecoverMsg::RequestedData {
                                data_slice,
                                data: self.read_checksum(data_slice),
                                dev_idx: self.dev_idx,
                            })
                            .unwrap();
                    }
                }
                Msg::HeadNodeDataRequest {
                    data_slice,
                    oneshot_send: oneshot_rec,
                } => {
                    let data = self.read_data(data_slice);
                    oneshot_rec.send(HeadNodeMsg { data_slice, data }).unwrap();
                }
                Msg::Shutdown => {
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    pub fn recover(&self, recover_rec: &Receiver<RecoverMsg<X>>, max_data_slice: usize) -> Result<()> {
        for current_data_slice in 0..max_data_slice + 1 {
            while !recover_rec.is_empty() {
                recover_rec.recv().unwrap();
            }
            for i in 0..C + D {
                if i != self.dev_idx {
                    self.coms[i]
                        .send(Msg::NeedRecover {
                            dev_idx: self.dev_idx,
                            data_slice: current_data_slice,
                        })?;
                }
            }
            let mut r_data = vec![];
            let mut r_check = vec![];
            let mut r_data_idx = vec![];
            let mut r_check_idx = vec![];
            while let Ok(msg) = recover_rec.recv() {
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

            let mut rec_data: [Box<[Galois; X]>; D] = r_data.try_into().unwrap();
            rec_matrix.gaussian_elimination(&mut rec_data);

            let data_check_idx = Self::data_check_idx(self.dev_idx, current_data_slice);
            if data_check_idx < D {
                self.write_data(current_data_slice, &rec_data[data_check_idx])
            } else {
                let checksum = self.vandermonde.mul_vec_at(&rec_data, data_check_idx - D);
                self.write_checksum(current_data_slice, &checksum);
            }
        }
        Ok(())
    }
}

pub struct HeadNode<const D: usize, const C: usize, const X: usize>
    where
        [(); C + D]:,
        [(); D + C]:,
        [(); C + C]:,
        [(); D + D]:,
{
    max_data_slices: usize,
    coms: [Sender<Msg<X>>; D + C],
    handles: [JoinHandle<()>; D + C],
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
            std::thread::Builder::new()
                .name(format!("thread{i}"))
                .spawn(move || {
                    let node = Node::new(path, i, v, c, rec_c);
                    let _ = node.start(r, rec_r);
                })
                .unwrap()
        });

        Self {
            max_data_slices: 0,
            handles,
            coms,
        }
    }

    fn add_data(&mut self, data: &[&[u8; X]; D], data_slice: usize) {
        self.max_data_slices = self.max_data_slices.max(data_slice);
        for data_idx in 0..D {
            let pdata = galois::from_slice_raw(data[data_idx]);
            let dev_idx = Self::dev_idx(data_slice, data_idx);
            self.coms[dev_idx]
                .send(Msg::NewData {
                    data_slice: data_slice,
                    data: pdata,
                })
                .unwrap()
        }
    }

    fn add_data_at(&mut self, data: &[u8; X], data_slice: usize, data_idx: usize) {
        self.max_data_slices = self.max_data_slices.max(data_slice);
        let data = galois::from_slice_raw(data);
        let dev_idx = Self::dev_idx(data_slice, data_idx);
        self.coms[dev_idx]
            .send(Msg::NewDataAt {
                data_slice,
                data,
            })
            .unwrap()
    }

    fn read_data(&self, data_slice: usize) -> [Box<[u8; X]>; D] {
        let receivers: [oneshot::Receiver<HeadNodeMsg<X>>; D] = std::array::from_fn(|i| {
            let dev_idx = Self::dev_idx(data_slice, i);
            let (rt, tx) = oneshot::channel();
            self.coms[dev_idx]
                .send(Msg::HeadNodeDataRequest {
                    data_slice,
                    oneshot_send: rt,
                })
                .unwrap();
            tx
        });

        let mut result = core::array::from_fn(|_| galois::as_bytes(galois::zeros()));
        for (i, receiver) in receivers.into_iter().enumerate() {
            let msg = receiver.recv().unwrap();
            assert_eq!(msg.data_slice, data_slice);
            result[i] = galois::as_bytes(msg.data);
        }
        result
    }

    fn read_data_at(&self, data_slice: usize, data_idx: usize) -> Box<[u8; X]> {
        let dev_idx = Self::dev_idx(data_slice, data_idx);
        let (rt, tx) = oneshot::channel();
        self.coms[dev_idx]
            .send(Msg::HeadNodeDataRequest {
                data_slice,
                oneshot_send: rt,
            })
            .unwrap();

        let msg = tx.recv().unwrap();
        assert_eq!(msg.data_slice, data_slice);
        galois::as_bytes(msg.data)
    }

    fn destroy_devices(&self, dev_idxs: &[usize]) {
        let mut txs = vec![];
        for dev_idx in dev_idxs {
            let (rt, tx) = oneshot::channel();
            txs.push(tx);
            self.coms[*dev_idx].send(Msg::DestroyStorage { oneshot_send: rt, max_data_slice: self.max_data_slices }).unwrap()
        }
        for tx in txs {
            tx.recv().unwrap()
        }
    }

    fn update_data(&self, data: &[u8; X], data_slice: usize, data_idx: usize) {
        let data = galois::from_slice(galois::from_bytes_ref(data));
        let dev_idx = Self::dev_idx(data_slice, data_idx);
        self.coms[dev_idx]
            .send(Msg::UpdateData { data_slice, data })
            .unwrap()
    }

    fn shutdown(self) {
        for dev_idx in 0..D + C {
            self.coms[dev_idx].send(Msg::Shutdown).unwrap()
        }
        for handle in self.handles {
            handle.join().unwrap();
        }
    }
}
