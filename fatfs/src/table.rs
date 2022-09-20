use core::borrow::BorrowMut;
use core::cmp;
use core::marker::PhantomData;

use crate::error::{Error, IoError};
use crate::fs::{FatType, FsStatusFlags};
use crate::io::{self, Read, ReadLeExt, Seek, Write, WriteLeExt};

struct Fat<S> {
    phantom: PhantomData<S>,
}

type Fat12 = Fat<u8>;
type Fat16 = Fat<u16>;
type Fat32 = Fat<u32>;

pub const RESERVED_FAT_ENTRIES: u32 = 2;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum FatValue {
    Free,
    Data(u32),
    Bad,
    EndOfChain,
}

trait FatTrait {
    fn get_raw<S, E>(fat: &mut S, cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>;

    fn get<S, E>(fat: &mut S, cluster: u32) -> Result<FatValue, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>;

    fn set_raw<S, E>(fat: &mut S, cluster: u32, raw_value: u32) -> Result<(), Error<E>>
    where
        S: Read + Write + Seek,
        E: IoError,
        Error<E>: From<S::Error>;

    fn set<S, E>(fat: &mut S, cluster: u32, value: FatValue) -> Result<(), Error<E>>
    where
        S: Read + Write + Seek,
        E: IoError,
        Error<E>: From<S::Error>;

    fn find_free<S, E>(fat: &mut S, start_cluster: u32, end_cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>;

    fn count_free<S, E>(fat: &mut S, end_cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>;
}

fn read_fat<S, E>(fat: &mut S, fat_type: FatType, cluster: u32) -> Result<FatValue, Error<E>>
where
    S: Read + Seek,
    E: IoError,
    Error<E>: From<S::Error>,
{
    match fat_type {
        FatType::Fat12 => Fat12::get(fat, cluster),
        FatType::Fat16 => Fat16::get(fat, cluster),
        FatType::Fat32 => Fat32::get(fat, cluster),
    }
}

fn write_fat<S, E>(fat: &mut S, fat_type: FatType, cluster: u32, value: FatValue) -> Result<(), Error<E>>
where
    S: Read + Write + Seek,
    E: IoError,
    Error<E>: From<S::Error>,
{
    match fat_type {
        FatType::Fat12 => Fat12::set(fat, cluster, value),
        FatType::Fat16 => Fat16::set(fat, cluster, value),
        FatType::Fat32 => Fat32::set(fat, cluster, value),
    }
}

fn get_next_cluster<S, E>(fat: &mut S, fat_type: FatType, cluster: u32) -> Result<Option<u32>, Error<E>>
where
    S: Read + Seek,
    E: IoError,
    Error<E>: From<S::Error>,
{
    let val = read_fat(fat, fat_type, cluster)?;
    match val {
        FatValue::Data(n) => Ok(Some(n)),
        _ => Ok(None),
    }
}

fn find_free_cluster<S, E>(
    fat: &mut S,
    fat_type: FatType,
    start_cluster: u32,
    end_cluster: u32,
) -> Result<u32, Error<E>>
where
    S: Read + Seek,
    E: IoError,
    Error<E>: From<S::Error>,
{
    match fat_type {
        FatType::Fat12 => Fat12::find_free(fat, start_cluster, end_cluster),
        FatType::Fat16 => Fat16::find_free(fat, start_cluster, end_cluster),
        FatType::Fat32 => Fat32::find_free(fat, start_cluster, end_cluster),
    }
}

pub fn alloc_cluster<S, E>(
    fat: &mut S,
    fat_type: FatType,
    prev_cluster: Option<u32>,
    hint: Option<u32>,
    total_clusters: u32,
) -> Result<u32, Error<E>>
where
    S: Read + Write + Seek,
    E: IoError,
    Error<E>: From<S::Error>,
{
    let end_cluster = total_clusters + RESERVED_FAT_ENTRIES;
    let start_cluster = match hint {
        Some(n) if n < end_cluster => n,
        _ => RESERVED_FAT_ENTRIES,
    };
    let new_cluster = match find_free_cluster(fat, fat_type, start_cluster, end_cluster) {
        Ok(n) => n,
        Err(_) if start_cluster > RESERVED_FAT_ENTRIES => {
            find_free_cluster(fat, fat_type, RESERVED_FAT_ENTRIES, start_cluster)?
        }
        Err(e) => return Err(e),
    };
    write_fat(fat, fat_type, new_cluster, FatValue::EndOfChain)?;
    if let Some(n) = prev_cluster {
        write_fat(fat, fat_type, n, FatValue::Data(new_cluster))?;
    }
    Ok(new_cluster)
}

pub fn read_fat_flags<S, E>(fat: &mut S, fat_type: FatType) -> Result<FsStatusFlags, Error<E>>
where
    S: Read + Seek,
    E: IoError,
    Error<E>: From<S::Error>,
{
    // check MSB (except in FAT12)
    let val = match fat_type {
        FatType::Fat12 => 0xFFF,
        FatType::Fat16 => Fat16::get_raw(fat, 1)?,
        FatType::Fat32 => Fat32::get_raw(fat, 1)?,
    };
    let dirty = match fat_type {
        FatType::Fat12 => false,
        FatType::Fat16 => val & (1 << 15) == 0,
        FatType::Fat32 => val & (1 << 27) == 0,
    };
    let io_error = match fat_type {
        FatType::Fat12 => false,
        FatType::Fat16 => val & (1 << 14) == 0,
        FatType::Fat32 => val & (1 << 26) == 0,
    };
    Ok(FsStatusFlags { dirty, io_error })
}

pub fn count_free_clusters<S, E>(fat: &mut S, fat_type: FatType, total_clusters: u32) -> Result<u32, Error<E>>
where
    S: Read + Seek,
    E: IoError,
    Error<E>: From<S::Error>,
{
    let end_cluster = total_clusters + RESERVED_FAT_ENTRIES;
    match fat_type {
        FatType::Fat12 => Fat12::count_free(fat, end_cluster),
        FatType::Fat16 => Fat16::count_free(fat, end_cluster),
        FatType::Fat32 => Fat32::count_free(fat, end_cluster),
    }
}

pub fn format_fat<S, E>(
    fat: &mut S,
    fat_type: FatType,
    media: u8,
    bytes_per_fat: u64,
    total_clusters: u32,
) -> Result<(), Error<E>>
where
    S: Read + Write + Seek,
    E: IoError,
    Error<E>: From<S::Error>,
{
    const BITS_PER_BYTE: u64 = 8;
    // init first two reserved entries to FAT ID
    match fat_type {
        FatType::Fat12 => {
            fat.write_u8(media)?;
            fat.write_u16_le(0xFFFF)?;
        }
        FatType::Fat16 => {
            fat.write_u16_le(u16::from(media) | 0xFF00)?;
            fat.write_u16_le(0xFFFF)?;
        }
        FatType::Fat32 => {
            fat.write_u32_le(u32::from(media) | 0xFFF_FF00)?;
            fat.write_u32_le(0xFFFF_FFFF)?;
        }
    };
    // mark entries at the end of FAT as used (after FAT but before sector end)
    let start_cluster = total_clusters + RESERVED_FAT_ENTRIES;
    let end_cluster = (bytes_per_fat * BITS_PER_BYTE / u64::from(fat_type.bits_per_fat_entry())) as u32;
    for cluster in start_cluster..end_cluster {
        write_fat(fat, fat_type, cluster, FatValue::EndOfChain)?;
    }
    // mark special entries 0x0FFFFFF0 - 0x0FFFFFFF as BAD if they exists on FAT32 volume
    if end_cluster > 0x0FFF_FFF0 {
        let end_bad_cluster = cmp::min(0x0FFF_FFFF + 1, end_cluster);
        for cluster in 0x0FFF_FFF0..end_bad_cluster {
            write_fat(fat, fat_type, cluster, FatValue::Bad)?;
        }
    }
    Ok(())
}

impl FatTrait for Fat12 {
    fn get_raw<S, E>(fat: &mut S, cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let fat_offset = cluster + (cluster / 2);
        fat.seek(io::SeekFrom::Start(u64::from(fat_offset)))?;
        let packed_val = fat.read_u16_le()?;
        Ok(u32::from(match cluster & 1 {
            0 => packed_val & 0x0FFF,
            _ => packed_val >> 4,
        }))
    }

    fn get<S, E>(fat: &mut S, cluster: u32) -> Result<FatValue, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let val = Self::get_raw(fat, cluster)?;
        Ok(match val {
            0 => FatValue::Free,
            0xFF7 => FatValue::Bad,
            0xFF8..=0xFFF => FatValue::EndOfChain,
            n => FatValue::Data(n),
        })
    }

    fn set<S, E>(fat: &mut S, cluster: u32, value: FatValue) -> Result<(), Error<E>>
    where
        S: Read + Write + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let raw_val = match value {
            FatValue::Free => 0,
            FatValue::Bad => 0xFF7,
            FatValue::EndOfChain => 0xFFF,
            FatValue::Data(n) => n,
        };
        Self::set_raw(fat, cluster, raw_val)
    }

    fn set_raw<S, E>(fat: &mut S, cluster: u32, raw_val: u32) -> Result<(), Error<E>>
    where
        S: Read + Write + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let fat_offset = cluster + (cluster / 2);
        fat.seek(io::SeekFrom::Start(u64::from(fat_offset)))?;
        let old_packed = fat.read_u16_le()?;
        fat.seek(io::SeekFrom::Start(u64::from(fat_offset)))?;
        let new_packed = match cluster & 1 {
            0 => (old_packed & 0xF000) | raw_val as u16,
            _ => (old_packed & 0x000F) | ((raw_val as u16) << 4),
        };
        fat.write_u16_le(new_packed)?;
        Ok(())
    }

    fn find_free<S, E>(fat: &mut S, start_cluster: u32, end_cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let mut cluster = start_cluster;
        let fat_offset = cluster + (cluster / 2);
        fat.seek(io::SeekFrom::Start(u64::from(fat_offset)))?;
        let mut packed_val = fat.read_u16_le()?;
        loop {
            let val = match cluster & 1 {
                0 => packed_val & 0x0FFF,
                _ => packed_val >> 4,
            };
            if val == 0 {
                return Ok(cluster);
            }
            cluster += 1;
            if cluster == end_cluster {
                return Err(Error::NotEnoughSpace);
            }
            packed_val = if cluster & 1 == 0 {
                fat.read_u16_le()?
            } else {
                let next_byte = fat.read_u8()?;
                (packed_val >> 8) | (u16::from(next_byte) << 8)
            };
        }
    }

    fn count_free<S, E>(fat: &mut S, end_cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let mut count = 0;
        let mut cluster = RESERVED_FAT_ENTRIES;
        fat.seek(io::SeekFrom::Start(u64::from(cluster * 3 / 2)))?;
        let mut prev_packed_val = 0_u16;
        while cluster < end_cluster {
            let res = match cluster & 1 {
                0 => fat.read_u16_le(),
                _ => fat.read_u8().map(u16::from),
            };
            let packed_val = match res {
                Err(err) => return Err(err.into()),
                Ok(n) => n,
            };
            let val = match cluster & 1 {
                0 => packed_val & 0x0FFF,
                _ => (packed_val << 8) | (prev_packed_val >> 12),
            };
            prev_packed_val = packed_val;
            if val == 0 {
                count += 1;
            }
            cluster += 1;
        }
        Ok(count)
    }
}

impl FatTrait for Fat16 {
    fn get_raw<S, E>(fat: &mut S, cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        fat.seek(io::SeekFrom::Start(u64::from(cluster * 2)))?;
        Ok(u32::from(fat.read_u16_le()?))
    }

    fn get<S, E>(fat: &mut S, cluster: u32) -> Result<FatValue, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let val = Self::get_raw(fat, cluster)?;
        Ok(match val {
            0 => FatValue::Free,
            0xFFF7 => FatValue::Bad,
            0xFFF8..=0xFFFF => FatValue::EndOfChain,
            n => FatValue::Data(n),
        })
    }

    fn set_raw<S, E>(fat: &mut S, cluster: u32, raw_value: u32) -> Result<(), Error<E>>
    where
        S: Read + Write + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        fat.seek(io::SeekFrom::Start(u64::from(cluster * 2)))?;
        fat.write_u16_le(raw_value as u16)?;
        Ok(())
    }

    fn set<S, E>(fat: &mut S, cluster: u32, value: FatValue) -> Result<(), Error<E>>
    where
        S: Read + Write + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let raw_value = match value {
            FatValue::Free => 0,
            FatValue::Bad => 0xFFF7,
            FatValue::EndOfChain => 0xFFFF,
            FatValue::Data(n) => n,
        };
        Self::set_raw(fat, cluster, raw_value)
    }

    fn find_free<S, E>(fat: &mut S, start_cluster: u32, end_cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let mut cluster = start_cluster;
        fat.seek(io::SeekFrom::Start(u64::from(cluster * 2)))?;
        while cluster < end_cluster {
            let val = fat.read_u16_le()?;
            if val == 0 {
                return Ok(cluster);
            }
            cluster += 1;
        }
        Err(Error::NotEnoughSpace)
    }

    fn count_free<S, E>(fat: &mut S, end_cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let mut count = 0;
        let mut cluster = RESERVED_FAT_ENTRIES;
        fat.seek(io::SeekFrom::Start(u64::from(cluster * 2)))?;
        while cluster < end_cluster {
            let val = fat.read_u16_le()?;
            if val == 0 {
                count += 1;
            }
            cluster += 1;
        }
        Ok(count)
    }
}

impl FatTrait for Fat32 {
    fn get_raw<S, E>(fat: &mut S, cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        fat.seek(io::SeekFrom::Start(u64::from(cluster * 4)))?;
        Ok(fat.read_u32_le()?)
    }

    fn get<S, E>(fat: &mut S, cluster: u32) -> Result<FatValue, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let val = Self::get_raw(fat, cluster)? & 0x0FFF_FFFF;
        Ok(match val {
            0 if (0x0FFF_FFF7..=0x0FFF_FFFF).contains(&cluster) => {
                let tmp = if cluster == 0x0FFF_FFF7 {
                    "BAD_CLUSTER"
                } else {
                    "end-of-chain"
                };
                warn!(
                    "cluster number {} is a special value in FAT to indicate {}; it should never be seen as free",
                    cluster, tmp
                );
                FatValue::Bad // avoid accidental use or allocation into a FAT chain
            }
            0 => FatValue::Free,
            0x0FFF_FFF7 => FatValue::Bad,
            0x0FFF_FFF8..=0x0FFF_FFFF => FatValue::EndOfChain,
            n if (0x0FFF_FFF7..=0x0FFF_FFFF).contains(&cluster) => {
                let tmp = if cluster == 0x0FFF_FFF7 {
                    "BAD_CLUSTER"
                } else {
                    "end-of-chain"
                };
                warn!("cluster number {} is a special value in FAT to indicate {}; hiding potential FAT chain value {} and instead reporting as a bad sector", cluster, tmp, n);
                FatValue::Bad // avoid accidental use or allocation into a FAT chain
            }
            n => FatValue::Data(n),
        })
    }

    fn set_raw<S, E>(fat: &mut S, cluster: u32, raw_value: u32) -> Result<(), Error<E>>
    where
        S: Read + Write + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        fat.seek(io::SeekFrom::Start(u64::from(cluster * 4)))?;
        fat.write_u32_le(raw_value)?;
        Ok(())
    }

    fn set<S, E>(fat: &mut S, cluster: u32, value: FatValue) -> Result<(), Error<E>>
    where
        S: Read + Write + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let old_reserved_bits = Self::get_raw(fat, cluster)? & 0xF000_0000;

        if value == FatValue::Free && cluster >= 0x0FFF_FFF7 && cluster <= 0x0FFF_FFFF {
            // NOTE: it is technically allowed for them to store FAT chain loops,
            //       or even have them all store value '4' as their next cluster.
            //       Some believe only FatValue::Bad should be allowed for this edge case.
            let tmp = if cluster == 0x0FFF_FFF7 {
                "BAD_CLUSTER"
            } else {
                "end-of-chain"
            };
            panic!(
                "cluster number {} is a special value in FAT to indicate {}; it should never be set as free",
                cluster, tmp
            );
        };
        let raw_val = match value {
            FatValue::Free => 0,
            FatValue::Bad => 0x0FFF_FFF7,
            FatValue::EndOfChain => 0x0FFF_FFFF,
            FatValue::Data(n) => n,
        };
        let raw_val = raw_val | old_reserved_bits; // must preserve original reserved values
        Self::set_raw(fat, cluster, raw_val)
    }

    fn find_free<S, E>(fat: &mut S, start_cluster: u32, end_cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let mut cluster = start_cluster;
        fat.seek(io::SeekFrom::Start(u64::from(cluster * 4)))?;
        while cluster < end_cluster {
            let val = fat.read_u32_le()? & 0x0FFF_FFFF;
            if val == 0 {
                return Ok(cluster);
            }
            cluster += 1;
        }
        Err(Error::NotEnoughSpace)
    }

    fn count_free<S, E>(fat: &mut S, end_cluster: u32) -> Result<u32, Error<E>>
    where
        S: Read + Seek,
        E: IoError,
        Error<E>: From<S::Error>,
    {
        let mut count = 0;
        let mut cluster = RESERVED_FAT_ENTRIES;
        fat.seek(io::SeekFrom::Start(u64::from(cluster * 4)))?;
        while cluster < end_cluster {
            let val = fat.read_u32_le()? & 0x0FFF_FFFF;
            if val == 0 {
                count += 1;
            }
            cluster += 1;
        }
        Ok(count)
    }
}

pub struct ClusterIterator<B, E, S = B> {
    fat: B,
    fat_type: FatType,
    cluster: Option<u32>,
    err: bool,
    // phantom is needed to add type bounds on the storage type
    phantom_s: PhantomData<S>,
    phantom_e: PhantomData<E>,
}

impl<B, E, S> ClusterIterator<B, E, S>
where
    B: BorrowMut<S>,
    E: IoError,
    S: Read + Write + Seek,
    Error<E>: From<S::Error>,
{
    pub fn new(fat: B, fat_type: FatType, cluster: u32) -> Self {
        Self {
            fat,
            fat_type,
            cluster: Some(cluster),
            err: false,
            phantom_s: PhantomData,
            phantom_e: PhantomData,
        }
    }

    pub fn truncate(&mut self) -> Result<u32, Error<E>> {
        if let Some(n) = self.cluster {
            // Move to the next cluster
            self.next();
            // Mark previous cluster as end of chain
            write_fat(self.fat.borrow_mut(), self.fat_type, n, FatValue::EndOfChain)?;
            // Free rest of chain
            self.free()
        } else {
            Ok(0)
        }
    }

    pub fn free(&mut self) -> Result<u32, Error<E>> {
        let mut num_free = 0;
        while let Some(n) = self.cluster {
            self.next();
            write_fat(self.fat.borrow_mut(), self.fat_type, n, FatValue::Free)?;
            num_free += 1;
        }
        Ok(num_free)
    }
}

impl<B, E, S> Iterator for ClusterIterator<B, E, S>
where
    B: BorrowMut<S>,
    E: IoError,
    S: Read + Write + Seek,
    Error<E>: From<S::Error>,
{
    type Item = Result<u32, Error<E>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.err {
            return None;
        }
        if let Some(current_cluster) = self.cluster {
            self.cluster = match get_next_cluster(self.fat.borrow_mut(), self.fat_type, current_cluster) {
                Ok(next_cluster) => next_cluster,
                Err(err) => {
                    self.err = true;
                    return Some(Err(err));
                }
            }
        }
        self.cluster.map(Ok)
    }
}
