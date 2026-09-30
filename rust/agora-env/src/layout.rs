//! A grid of terrain kinds (SPEC-006).

use thiserror::Error;

use crate::kinds::KindId;

/// Why a layout could not be built.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LayoutError {
    #[error("layout dimensions must be non-zero, got {width} x {height}")]
    Empty { width: u32, height: u32 },
    #[error("layout dimensions {width} x {height} are too large")]
    TooLarge { width: u32, height: u32 },
    #[error("a {width} x {height} layout needs {expected} cells, got {actual}")]
    WrongCellCount {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
}

/// The terrain kind of every cell in a bounded grid.
///
/// Cells are in row-major order from the south-west corner: the cell at `(x, y)` is at index
/// `y * width + x`, where `y` grows north, as in the simulation's coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    width: u32,
    height: u32,
    cells: Vec<KindId>,
}

impl Layout {
    /// A layout from its cells, in the order described on [`Layout`].
    pub fn new(width: u32, height: u32, cells: Vec<KindId>) -> Result<Self, LayoutError> {
        let expected = cell_count(width, height)?;
        if cells.len() != expected {
            return Err(LayoutError::WrongCellCount {
                width,
                height,
                expected,
                actual: cells.len(),
            });
        }
        Ok(Self {
            width,
            height,
            cells,
        })
    }

    /// A layout with every cell the same kind.
    pub fn filled(width: u32, height: u32, kind: KindId) -> Result<Self, LayoutError> {
        let cells = vec![kind; cell_count(width, height)?];
        Ok(Self {
            width,
            height,
            cells,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The kind at `(x, y)`, or `None` outside the grid.
    pub fn get(&self, x: u32, y: u32) -> Option<&KindId> {
        (x < self.width && y < self.height)
            .then(|| &self.cells[y as usize * self.width as usize + x as usize])
    }

    /// Every cell, in the order described on [`Layout`].
    pub fn cells(&self) -> &[KindId] {
        &self.cells
    }
}

fn cell_count(width: u32, height: u32) -> Result<usize, LayoutError> {
    if width == 0 || height == 0 {
        return Err(LayoutError::Empty { width, height });
    }
    (width as usize)
        .checked_mul(height as usize)
        .ok_or(LayoutError::TooLarge { width, height })
}
