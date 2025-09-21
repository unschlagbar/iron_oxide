use crate::graphics::UiInstance;

#[derive(Default)]
pub struct DrawData {
    pub groups: Vec<DrawGroup>,
}

impl DrawData {
    pub fn get_group(&mut self, descriptor: u32, material: u32) -> &mut InstanceData {
        if let Some(idx) = self
            .groups
            .iter()
            .position(|x| x.descriptor == descriptor && x.data.material_idx() == material)
        {
            &mut self.groups[idx].data
        } else {
            // Find the last group with the same material
            let insert_pos = self
                .groups
                .iter()
                .rposition(|x| x.data.material_idx() == material)
                .map(|idx| idx + 1)
                .unwrap_or(0);
            self.groups.insert(
                insert_pos,
                DrawGroup {
                    descriptor,
                    data: InstanceData::from_idx(material),
                },
            );
            &mut self.groups[insert_pos].data
        }
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

pub struct DrawGroup {
    pub descriptor: u32,
    pub data: InstanceData,
}

pub enum InstanceData {
    Basic(Vec<UiInstance>),
    BasicCorner(Vec<UiInstance>),
}

impl InstanceData {
    pub fn material_idx(&self) -> u32 {
        match self {
            Self::Basic(_) => 0,
            Self::BasicCorner(_) => 1,
        }
    }

    pub fn from_idx(material: u32) -> Self {
        match material {
            0 => Self::Basic(Vec::new()),
            1 => Self::BasicCorner(Vec::new()),
            _ => unreachable!(),
        }
    }
}
