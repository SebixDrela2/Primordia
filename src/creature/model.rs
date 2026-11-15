//! A model of a mind: the weights, the biases, and the genes that go
//! with them.
//!
//! The model is what the learner writes and what the run reads. It is
//! a small file of numbers, and it is written so that a later version
//! of the game can still read it: the head of the file says what
//! version it is, and a file of the wrong version is refused instead
//! of being read as nonsense.
//!
//! The file holds:
//!
//! - the shape of the network (the sizes of its layers),
//! - the weights and the biases of every layer,
//! - the instinct that comes before the network is asked (the skip
//!   connections and the bias that goes with them),
//! - the genes of the creature the network belongs to, so that the
//!   traits and the body come with it,
//! - what the creature had learnt (its culture) and how well it did.

use std::io::{Read, Write};
use std::path::Path;

use super::Net;

/// The head of every model file.
const MAGIC: &[u8; 8] = b"PRIMMOD1";

/// The version of the format this code writes.
const VERSION: u32 = 1;

/// A model of a mind.
#[derive(Clone, Debug)]
pub struct Model {
    /// The size of every layer of the network.
    pub sizes: Vec<usize>,
    /// The weights of every layer.
    pub weights: Vec<Vec<f32>>,
    /// The bias of every layer.
    pub biases: Vec<Vec<f32>>,
    /// The function the layers use.
    pub activation: u8,
    /// Which of the three shapes of brain it is.
    pub arch: u8,
    /// The instinct that comes before the network is asked.
    pub skip: Vec<f32>,
    /// The bias of the instinct.
    pub skip_up: f32,
    /// The genes of the creature.
    pub genes: [f32; 32],
    /// The traits that go with the genes.
    pub traits: u32,
    /// What the creature had learnt.
    pub culture: u16,
    /// The size of the body.
    pub size: f32,
    /// The colour of the species.
    pub color: [u8; 3],
    /// How many generations it is old.
    pub generation: u32,
    /// How well it did.
    pub fitness: f32,
}

impl Model {
    /// A model of a net, with the genes that go with it.
    #[must_use]
    pub fn from_net(net: &Net, genes: [f32; 32], traits: u32, culture: u16, size: f32, color: [u8; 3]) -> Self {
        let (weights, biases) = net.parts();
        Model {
            sizes: net.sizes().to_vec(),
            weights: weights.to_vec(),
            biases: biases.to_vec(),
            activation: net.activation,
            arch: net.arch,
            skip: net.skip.clone(),
            skip_up: net.skip_up,
            genes,
            traits,
            culture,
            size,
            color,
            generation: 0,
            fitness: 0.0,
        }
    }

    /// The net of the model: the same mind, ready to be used.
    #[must_use]
    pub fn to_net(&self) -> Net {
        Net::from_parts(
            &self.sizes,
            self.weights.clone(),
            self.biases.clone(),
            self.activation,
            self.arch,
            self.skip.clone(),
            self.skip_up,
        )
    }

    /// How many numbers the network has.
    #[must_use]
    pub fn weight_count(&self) -> usize {
        self.weights.iter().map(Vec::len).sum()
    }

    /// Write the model to a file.
    ///
    /// # Errors
    ///
    /// It fails when the file cannot be written.
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.push(self.activation);
        bytes.push(self.arch);
        bytes.extend_from_slice(&0u16.to_le_bytes());

        bytes.extend_from_slice(&(self.sizes.len() as u32).to_le_bytes());
        for size in &self.sizes {
            bytes.extend_from_slice(&(*size as u32).to_le_bytes());
        }
        // The weights, layer by layer.
        bytes.extend_from_slice(&(self.weights.len() as u32).to_le_bytes());
        for layer in &self.weights {
            bytes.extend_from_slice(&(layer.len() as u32).to_le_bytes());
            for value in layer {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        // The bias, layer by layer.
        bytes.extend_from_slice(&(self.biases.len() as u32).to_le_bytes());
        for layer in &self.biases {
            bytes.extend_from_slice(&(layer.len() as u32).to_le_bytes());
            for value in layer {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes.extend_from_slice(&(self.skip.len() as u32).to_le_bytes());
        for value in &self.skip {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&self.skip_up.to_le_bytes());
        for value in self.genes {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&self.traits.to_le_bytes());
        bytes.extend_from_slice(&self.culture.to_le_bytes());
        bytes.extend_from_slice(&self.size.to_le_bytes());
        bytes.extend_from_slice(&self.color);
        bytes.extend_from_slice(&self.generation.to_le_bytes());
        bytes.extend_from_slice(&self.fitness.to_le_bytes());

        let mut file = std::fs::File::create(Path::new(path))?;
        file.write_all(&bytes)?;
        file.flush()
    }

    /// Read a model from a file.
    ///
    /// # Errors
    ///
    /// It fails when the file cannot be read, or when it is not a model
    /// of this version.
    pub fn load(path: &str) -> std::io::Result<Model> {
        let mut file = std::fs::File::open(Path::new(path))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Model::from_bytes(&bytes)
    }

    /// Read a model out of the bytes of a file.
    ///
    /// # Errors
    ///
    /// It fails when the bytes are not a model of this version.
    pub fn from_bytes(bytes: &[u8]) -> std::io::Result<Model> {
        let mut reader = Reader { bytes, at: 0 };

        let head = reader.take(MAGIC.len())?;
        if head != MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "this is not a model file",
            ));
        }
        let version = reader.number()?;
        if version != VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("the model file is version {version}, and this game reads version {VERSION}"),
            ));
        }
        let activation = reader.byte()?;
        let arch = reader.byte()?;
        let _pad = reader.take(2)?;

        let count = reader.number()? as usize;
        let mut sizes = Vec::with_capacity(count);
        for _ in 0..count {
            sizes.push(reader.number()? as usize);
        }

        let weights = reader.layers()?;
        let biases = reader.layers()?;

        let skip_len = reader.number()? as usize;
        let mut skip = Vec::with_capacity(skip_len);
        for _ in 0..skip_len {
            skip.push(reader.decimal()?);
        }
        let skip_up = reader.decimal()?;

        let mut genes = [0.0f32; 32];
        for gene in &mut genes {
            *gene = reader.decimal()?;
        }
        let traits = reader.number()?;
        let culture = reader.short()?;
        let size = reader.decimal()?;
        let mut color = [0u8; 3];
        color.copy_from_slice(reader.take(3)?);
        let generation = reader.number()?;
        let fitness = reader.decimal()?;

        Ok(Model {
            sizes,
            weights,
            biases,
            activation,
            arch,
            skip,
            skip_up,
            genes,
            traits,
            culture,
            size,
            color,
            generation,
            fitness,
        })
    }
}

/// A little reader: the bytes of a model file, one number at a time.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> std::io::Result<&'a [u8]> {
        if self.at + n > self.bytes.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "the model file ends too early",
            ));
        }
        let slice = &self.bytes[self.at..self.at + n];
        self.at += n;
        Ok(slice)
    }

    fn number(&mut self) -> std::io::Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn short(&mut self) -> std::io::Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn byte(&mut self) -> std::io::Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn decimal(&mut self) -> std::io::Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn layers(&mut self) -> std::io::Result<Vec<Vec<f32>>> {
        let count = self.number()? as usize;
        let mut layers = Vec::with_capacity(count);
        for _ in 0..count {
            let len = self.number()? as usize;
            let mut layer = Vec::with_capacity(len);
            for _ in 0..len {
                layer.push(self.decimal()?);
            }
            layers.push(layer);
        }
        Ok(layers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::{brain_sizes, Rng};

    /// A model must survive the trip to a file and back.
    #[test]
    fn a_model_survives_the_trip() {
        let mut rng = Rng::new(7);
        let net = Net::random(&brain_sizes(), &mut rng);
        let model = Model::from_net(&net, [0.25; 32], 3, 9, 1.1, [10, 20, 30]);
        let path = std::env::temp_dir().join("primordia_model_test.bin");
        let path = path.to_string_lossy().to_string();
        model.save(&path).expect("the model should save");
        let read = Model::load(&path).expect("the model should load");
        let _ = std::fs::remove_file(&path);

        assert_eq!(read.sizes, model.sizes);
        assert_eq!(read.weights, model.weights);
        assert_eq!(read.biases, model.biases);
        assert_eq!(read.skip, model.skip);
        assert!(read
            .genes
            .iter()
            .zip(model.genes.iter())
            .all(|(a, b)| (a - b).abs() < 0.0001));
        assert_eq!(read.traits, model.traits);
        assert_eq!(read.culture, model.culture);
        assert_eq!(read.color, model.color);
    }

    /// A file that is not a model must be refused.
    #[test]
    fn a_strange_file_is_refused() {
        let err = Model::from_bytes(b"not a model at all").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
