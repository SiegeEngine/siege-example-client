
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::fs::File;
use std::path::Path;
use chashmap::{CHashMap, ReadGuard};
use siege_font::FontAtlas;
use errors::*;

mod geom;
pub use self::geom::{Point, Dim, Anchor, Coord, AbsRect, Rect, RectX, RectY};

mod window;
pub use self::window::UiWindow;

mod image;
pub use self::image::{UiImage,
                      WINDOW_TL_CORNER, WINDOW_TR_CORNER,
                      WINDOW_BL_CORNER, WINDOW_BR_CORNER,
                      WINDOW_TOP, WINDOW_BOTTOM,
                      WINDOW_LEFT, WINDOW_RIGHT,
                      WINDOW_PANE,
                      WINDOW_L_RULE, WINDOW_RULE, WINDOW_R_RULE};

mod text;
pub use self::text::{TextColor, Font, TextLine};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(pub usize);

#[derive(Debug, Clone)]
pub enum UiElement {
    Window(UiWindow),
    Image(UiImage),
    Text(TextLine),
}

impl UiElement {
    pub fn get_rect(&self) -> Option<Rect> {
        match *self {
            UiElement::Window(ref win) => Some(win.rect.clone()),
            UiElement::Image(_) => None, // maybe FIXME?
            UiElement::Text(_) => None,
        }
    }

    pub fn get_alpha(&self) -> f32 {
        match *self {
            UiElement::Window(ref win) => win.child_alpha,
            UiElement::Image(_) => 1.0,
            UiElement::Text(ref tl) => tl.alpha as f32 / 255.0,
        }
    }
}

/// This is only public so that ReadGuards passed along dont leak private
/// types.  Please dont use it outside of this module, except to reference
/// the element.
pub struct UiNode {
    pub element: UiElement,
    pub children: Vec<Handle>,
}

pub struct Ui {
    atlas: FontAtlas,
    text_is_dirty: AtomicBool,
    win_is_dirty: AtomicBool,
    image_is_dirty: AtomicBool,
    map: CHashMap<Handle, UiNode>,
    roots: RwLock<Vec<Handle>>,
    next_unused_id: AtomicUsize,
}

impl Ui {
    pub fn new(asset_path: &Path) -> Result<Ui>
    {
        let atlas: FontAtlas = {
            let file = File::open(asset_path.join("fonts")
                                  .join("Gudea-Regular.bin"))?;
            ::bincode::deserialize_from(&file)?
        };

        Ok(Ui {
            atlas: atlas,
            text_is_dirty: AtomicBool::new(true),
            win_is_dirty: AtomicBool::new(true),
            image_is_dirty: AtomicBool::new(true),
            map: CHashMap::new(),
            roots: RwLock::new(vec![]),
            next_unused_id: AtomicUsize::new(1),
        })
    }

    pub fn add_element(
        &self,
        element: UiElement,
        parent: Option<Handle>) -> Option<Handle>
    {
        // Allocate an id
        let new_id = Handle(self.next_unused_id.fetch_add(1, Ordering::Relaxed));

        // Add to parent's children
        if let Some(ref p) = parent {
            let mut parent_ref = match self.map.get_mut(p) {
                Some(r) => r,
                None => {
                    // undo the id allocation, because we failed
                    let _ = self.next_unused_id.fetch_sub(1, Ordering::Relaxed);
                    return None;
                }
            };
            parent_ref.children.push(new_id);
        } else {
            let mut roots = self.roots.write().unwrap();
            roots.push(new_id);
        }

        match element {
            UiElement::Text(_) => self.text_is_dirty.store(true, Ordering::Relaxed),
            UiElement::Window(_) => self.win_is_dirty.store(true, Ordering::Relaxed),
            UiElement::Image(_) => self.image_is_dirty.store(true, Ordering::Relaxed),
        }

        // Insert into the map
        self.map.insert(new_id, UiNode {
            element: element,
            children: vec![]
        });

        Some(new_id)
    }

    pub fn upsert<I,U>(
        &self,
        id: Handle,
        insert: I,
        update: U)
    where I: FnOnce() -> UiElement,
          U: FnOnce(&mut UiElement)
    {
        let iinsert = || { UiNode { element: insert(), children: vec![] } };
        let iupdate = |n: &mut UiNode| { update(&mut n.element); };
        self.map.upsert(id, iinsert, iupdate);
        self.text_is_dirty.store(true, Ordering::Relaxed); // because it might be text
    }

    pub fn set_text(&self, id: Handle, text: String) -> bool
    {
        use std::ops::DerefMut;
        let mut guard = match self.map.get_mut(&id) {
            Some(guard) => guard,
            None => return false,
        };
        let node: &mut UiNode = guard.deref_mut();
        if let UiElement::Text(ref mut textline) = node.element {
            textline.text = text;
            self.text_is_dirty.store(true, Ordering::Relaxed);
            return true;
        }
        false
    }

    pub fn is_text_dirty(&self) -> bool
    {
        self.text_is_dirty.load(Ordering::Relaxed)
    }

    pub fn is_win_dirty(&self) -> bool
    {
        self.win_is_dirty.load(Ordering::Relaxed)
    }

    pub fn is_image_dirty(&self) -> bool
    {
        self.image_is_dirty.load(Ordering::Relaxed)
    }

    pub fn clear_text_dirty(&self) {
        self.text_is_dirty.store(false, Ordering::Relaxed);
    }

    pub fn clear_win_dirty(&self) {
        self.win_is_dirty.store(false, Ordering::Relaxed);
    }

    pub fn clear_image_dirty(&self) {
        self.image_is_dirty.store(false, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn get_pixel_length(&self, line: &TextLine) -> f32
    {
        let atlas = &self.atlas;
        let scale = line.lineheight as f32 / atlas.line_height;
        let mut cursor: f32 = 0.0;
        for ch in line.text.chars() {
            let cinfo = match atlas.map.get(&ch) {
                Some(cinfo) => cinfo,
                None => continue, // FIXME: use a placeholder character
            };
            cursor += cinfo.post_draw_advance * scale;
        }
        cursor
    }

    pub fn walk<'a>(&'a self, width: f32, height: f32) -> Walker<'a>
    {
        let screen = AbsRect {
            x: 0.0,
            y: 0.0,
            width: width,
            height: height,
        };

        // We do all the walking up front.
        let nodes = Walker::walk(self, screen);

        Walker {
            ui: self,
            cur: 0,
            nodes: nodes
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub handle: Handle,
    pub rect: AbsRect,
    pub depth: usize,
    pub alpha: f32
}

pub struct Walker<'a> {
    ui: &'a Ui,
    cur: usize,
    nodes: Vec<NodeInfo>, // already walked in order
}

impl<'a> Iterator for Walker<'a> {
    type Item = (NodeInfo, ReadGuard<'a, Handle, UiNode>);

    fn next(&mut self) -> Option<(NodeInfo, ReadGuard<'a, Handle, UiNode>)> {
        if self.cur >= self.nodes.len() {
            return None;
        }
        let thisnode = self.nodes[self.cur].clone();
        self.cur += 1;
        let guard = self.ui.map.get(&thisnode.handle).unwrap();
        Some((thisnode, guard))
    }
}

impl<'a> Walker<'a> {
    fn walk(ui: &'a Ui, screen: AbsRect) -> Vec<NodeInfo>
    {
        let rootlen = {
            let roots = ui.roots.read().unwrap();
            roots.len()
        };
        let mut output: Vec<NodeInfo> = vec![];
        let mut maxdepth = 0; // each subsequent tree is at an entire new depth
        for rootindex in 0..rootlen {
            let nid = {
                let roots = ui.roots.read().unwrap();
                roots[rootindex]
            };
            let readguard = ui.map.get(&nid).unwrap();
            let root_node = &readguard;
            let root_nodeinfo = NodeInfo {
                handle: nid,
                rect: match root_node.element.get_rect() {
                    Some(vp) => vp.absolute(&screen),
                    None => screen.clone()
                },
                depth: maxdepth,
                alpha: root_node.element.get_alpha(),
            };
            let children = Self::walktree(ui, root_nodeinfo, root_node);
            maxdepth = children.iter().fold(maxdepth, |sofar, ref v| sofar.max(v.depth)) + 1;
            output.extend( children );
        }
        output
    }

    fn walktree(ui: &'a Ui,
                parent_nodeinfo: NodeInfo,
                parent_node: &'a UiNode) -> Vec<NodeInfo>
    {
        let mut output: Vec<NodeInfo> = vec![];
        for nid in &parent_node.children {
            let readguard = ui.map.get(nid).unwrap();
            let child_node = &readguard;
            let child_nodeinfo = NodeInfo {
                handle: *nid,
                rect: match child_node.element.get_rect() {
                    Some(vp) => vp.absolute(&parent_nodeinfo.rect),
                    None => parent_nodeinfo.rect.clone()
                },
                depth: parent_nodeinfo.depth+1,
                alpha: parent_nodeinfo.alpha * child_node.element.get_alpha()
            };
            output.extend( Self::walktree(ui, child_nodeinfo, child_node) );
        }
        // do parent _after_ children (front to back avoids redrawing)
        output.push(parent_nodeinfo.clone());
        output
    }
}

/*

#[cfg(test)]
mod test {
    use super::{UiElement, UiNode, UiWindow, Ui, Rect};
    use siege_math::Vec4;

    #[test]
    fn test_ui_walker() {
        let mut ui = Ui::new();
        let element1 = UiElement::Window(UiWindow::new(
            Default::default(), [1.0, 1.0, 1.0, 1.0]));
        let element2 = UiElement::Window(UiWindow::new(
            Rect::new((0.1, 0.1), (0.9, 0.9)),
            [1.0, 1.0, 1.0, 1.0]));

        let a = ui.add_element(element1.clone(), None).unwrap();
        let b = ui.add_element(element1.clone(), Some(a)).unwrap();
        let c = ui.add_element(element1.clone(), Some(b)).unwrap();
        let d = ui.add_element(element2.clone(), Some(b)).unwrap();
        let e = ui.add_element(element1.clone(), Some(a)).unwrap();
        let f = ui.add_element(element2.clone(), Some(e)).unwrap();
        let g = ui.add_element(element2.clone(), Some(f)).unwrap();

        let h = ui.add_element(element1.clone(), None).unwrap();
        let i = ui.add_element(element1.clone(), Some(h)).unwrap();

        let mut iter = ui.iter();

        let (depth, _, nid, _) = iter.next().unwrap();
        assert_eq!(depth, 0);
        assert_eq!(nid, a);

        let (depth, _, nid, _) = iter.next().unwrap();
        assert_eq!(depth, 1);
        assert_eq!(nid, b);

        let (depth, _, nid, _) = iter.next().unwrap();
        assert_eq!(depth, 2);
        assert_eq!(nid, c);

        let (depth, _, nid, _) = iter.next().unwrap();
        assert_eq!(depth, 2);
        assert_eq!(nid, d);

        let (depth, _, nid, _) = iter.next().unwrap();
        assert_eq!(depth, 1);
        assert_eq!(nid, e);

        let (depth, _, nid, _) = iter.next().unwrap();
        assert_eq!(depth, 2);
        assert_eq!(nid, f);

        let (depth, rect, nid, _) = iter.next().unwrap();
        assert_eq!(depth, 3);
        assert!(rect.x1 > 0.17);
        assert!(rect.x1 < 0.19);
        assert!(rect.x2 > 0.81);
        assert!(rect.x2 < 0.83);
        assert!(rect.y1 > 0.17);
        assert!(rect.y1 < 0.19);
        assert!(rect.y2 > 0.81);
        assert!(rect.y2 < 0.83);
        assert_eq!(nid, g);

        let (depth, _, nid, _) = iter.next().unwrap();
        assert_eq!(depth, 0);
        assert_eq!(nid, h);

        let (depth, _, nid, _) = iter.next().unwrap();
        assert_eq!(depth, 1);
        assert_eq!(nid, i);

        assert!(iter.next().is_none());
    }
}
*/
