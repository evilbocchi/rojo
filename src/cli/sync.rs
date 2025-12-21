use std::{
    io::{BufReader, BufWriter, Write as _},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context as _};
use clap::Parser;
use fs_err::File;
use memofs::Vfs;
use rbx_dom_weak::{InstanceBuilder, WeakDom};

use crate::{
    project::Project,
    snapshot::{apply_patch_set, compute_patch_set, InstanceContext, InstanceSnapshot, RojoTree},
    snapshot_middleware::snapshot_from_vfs,
};

use super::resolve_path;

const UNKNOWN_INPUT_KIND_ERR: &str = "Could not detect what kind of file was inputted. \
                                       Expected input file to end in .rbxl, .rbxlx, .rbxm, or .rbxmx.";
const UNKNOWN_OUTPUT_KIND_ERR: &str = "Could not detect what kind of file to output. \
                                       Expected output file to end in .rbxl, .rbxlx, .rbxm, or .rbxmx.";

/// Merges a Rojo project into an existing Roblox place/model file.
///
/// Unlike `rojo build`, this command starts from an existing input file and
/// applies Rojo's changes on top, preserving instances and properties that are
/// not managed by the Rojo project.
#[derive(Debug, Parser)]
pub struct SyncCommand {
    /// Path to the project to build. Defaults to the current directory.
    #[clap(default_value = "")]
    pub project: PathBuf,

    /// Path to the Roblox file to merge into.
    #[clap(long, short)]
    pub input: PathBuf,

    /// Where to output the merged result.
    #[clap(long, short)]
    pub output: PathBuf,
}

impl SyncCommand {
    pub fn run(self) -> anyhow::Result<()> {
        let project_path = resolve_path(&self.project);

        let input_kind = FileKind::from_path(&self.input).context(UNKNOWN_INPUT_KIND_ERR)?;
        let output_kind = FileKind::from_path(&self.output).context(UNKNOWN_OUTPUT_KIND_ERR)?;

        // Build the desired project snapshot using the same middleware pipeline
        // as `serve`/`build`.
        let vfs = Vfs::new_default();
        let root_project = Project::load_initial_project(&vfs, project_path.as_ref())?;
        let instance_context =
            InstanceContext::with_emit_legacy_scripts(root_project.emit_legacy_scripts);
        let desired_snapshot = snapshot_from_vfs(&instance_context, &vfs, project_path.as_ref())?;

        // Load the input file into a RojoTree.
        let dom_old = read_dom(&self.input, input_kind)?;
        let old_root = dom_old.root_ref();
        let mut tree_old = RojoTree::new(InstanceSnapshot::from_tree(dom_old, old_root));

        let root_id = tree_old.get_root_id();
        let mut patch_set = compute_patch_set(desired_snapshot, &tree_old, root_id);

        // Preserve existing content:
        // - Do not delete any instances that exist in the input file
        // - Do not remove any properties that exist in the input file
        patch_set.removed_instances.clear();
        for update in &mut patch_set.updated_instances {
            update
                .changed_properties
                .retain(|_, value| value.is_some());
        }

        apply_patch_set(&mut tree_old, patch_set);

        write_tree_to_file(&tree_old, &self.output, output_kind)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Rbxmx,
    Rbxlx,
    Rbxm,
    Rbxl,
}

impl FileKind {
    fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;

        match extension {
            "rbxlx" => Some(FileKind::Rbxlx),
            "rbxmx" => Some(FileKind::Rbxmx),
            "rbxl" => Some(FileKind::Rbxl),
            "rbxm" => Some(FileKind::Rbxm),
            _ => None,
        }
    }
}

fn xml_decode_config() -> rbx_xml::DecodeOptions<'static> {
    rbx_xml::DecodeOptions::new().property_behavior(rbx_xml::DecodePropertyBehavior::ReadUnknown)
}

fn xml_encode_config() -> rbx_xml::EncodeOptions<'static> {
    rbx_xml::EncodeOptions::new().property_behavior(rbx_xml::EncodePropertyBehavior::WriteUnknown)
}

fn read_dom(path: &Path, file_kind: FileKind) -> anyhow::Result<WeakDom> {
    let content = BufReader::new(File::open(path)?);
    match file_kind {
        FileKind::Rbxl => rbx_binary::from_reader(content).with_context(|| {
            format!(
                "Could not deserialize binary place file at {}",
                path.display()
            )
        }),
        FileKind::Rbxlx => rbx_xml::from_reader(content, xml_decode_config())
            .with_context(|| format!("Could not deserialize XML place file at {}", path.display())),
        FileKind::Rbxm => {
            let temp_tree = rbx_binary::from_reader(content).with_context(|| {
                format!(
                    "Could not deserialize binary model file at {}",
                    path.display()
                )
            })?;

            process_model_dom(temp_tree)
        }
        FileKind::Rbxmx => {
            let temp_tree = rbx_xml::from_reader(content, xml_decode_config()).with_context(|| {
                format!("Could not deserialize XML model file at {}", path.display())
            })?;
            process_model_dom(temp_tree)
        }
    }
}

fn process_model_dom(dom: WeakDom) -> anyhow::Result<WeakDom> {
    let temp_children = dom.root().children();
    if temp_children.len() == 1 {
        let real_root = dom.get_by_ref(temp_children[0]).unwrap();
        let mut new_tree = WeakDom::new(InstanceBuilder::new(real_root.class));
        for (name, property) in &real_root.properties {
            new_tree
                .root_mut()
                .properties
                .insert(*name, property.to_owned());
        }

        let children = dom.clone_multiple_into_external(real_root.children(), &mut new_tree);
        for child in children {
            new_tree.transfer_within(child, new_tree.root_ref());
        }
        Ok(new_tree)
    } else {
        bail!(
            "Rojo does not currently support models with more than one Instance at the Root!"
        );
    }
}

fn write_tree_to_file(tree: &RojoTree, output: &Path, kind: FileKind) -> anyhow::Result<()> {
    let mut file = BufWriter::new(File::create(output)?);

    let root_id = tree.get_root_id();

    match kind {
        FileKind::Rbxm => {
            rbx_binary::to_writer(&mut file, tree.inner(), &[root_id])?;
        }
        FileKind::Rbxl => {
            let root_instance = tree
                .inner()
                .get_by_ref(root_id)
                .expect("tree root should exist");
            let top_level_ids = root_instance.children();

            rbx_binary::to_writer(&mut file, tree.inner(), top_level_ids)?;
        }
        FileKind::Rbxmx => {
            rbx_xml::to_writer(&mut file, tree.inner(), &[root_id], xml_encode_config())?;
        }
        FileKind::Rbxlx => {
            let root_instance = tree
                .inner()
                .get_by_ref(root_id)
                .expect("tree root should exist");
            let top_level_ids = root_instance.children();

            rbx_xml::to_writer(&mut file, tree.inner(), top_level_ids, xml_encode_config())?;
        }
    }

    file.flush()?;
    Ok(())
}


