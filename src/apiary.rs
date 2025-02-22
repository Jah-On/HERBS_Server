use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Apiary {
	name:     String,
	gateways: HashMap<String, Vec<String>>
}

impl Apiary {
	pub fn has_gateway(self: &Self, id: String) -> bool {
		return self.gateways.contains_key(&id);
	}

	pub fn get_bridges(self: &Self) -> Vec<String> {
		let mut collected: Vec<String> = Vec::new();

		for bridges in self.gateways.values() {
			for bridge in bridges {
				collected.push(bridge.clone());
			}
		}

		return collected;
	}

	pub fn has_bridge(self: &Self, bridge_id: String) -> bool {
		for nodes in self.gateways.values() {
			for node in nodes {
				if bridge_id == *node { return true; }
			}
		}

		return false;
	}
}
