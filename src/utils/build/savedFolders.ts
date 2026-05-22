import {
  type Folder,
  newId,
  readLibrary,
  writeLibrary,
} from './savedBuilds'

export function listFolders(): Folder[] {
  // Returns every persisted folder in storage order. Used by the build library to render the folder tree.
  return readLibrary().folders.slice()
}

export function createFolder(name: string, parentId: string | null): Folder {
  // Creates a new folder under `parentId` (validated to be an existing folder, otherwise treated as a top-level folder) and persists it. Used by the library's "New folder" action.
  const library = readLibrary()
  const validParent =
    parentId !== null && library.folders.some((f) => f.id === parentId)
      ? parentId
      : null
  const folder: Folder = {
    id: newId('f'),
    name,
    parentId: validParent,
    createdAt: new Date().toISOString(),
  }
  library.folders.push(folder)
  writeLibrary(library)
  return folder
}

export function renameFolder(folderId: string, name: string): Folder | null {
  // Renames a folder by id, returning null when it does not exist. Used by the library's folder rename action.
  const library = readLibrary()
  const folder = library.folders.find((f) => f.id === folderId)
  if (!folder) return null
  folder.name = name
  writeLibrary(library)
  return folder
}

function collectDescendants(
  rootId: string,
  folders: Folder[],
): Set<string> {
  // Returns the set of folder ids in the subtree rooted at `rootId` (inclusive). Used by `deleteFolder` for cascade deletion.
  const out = new Set<string>([rootId])
  let added = true
  while (added) {
    added = false
    for (const f of folders) {
      if (f.parentId !== null && out.has(f.parentId) && !out.has(f.id)) {
        out.add(f.id)
        added = true
      }
    }
  }
  return out
}

export function deleteFolder(
  folderId: string,
  opts: { cascade: boolean },
): void {
  // Deletes a folder. With `cascade: false` (the safe default) child folders are reparented onto the deleted folder's parent and its builds are unfiled (folderId set to null). With `cascade: true` the whole subtree of folders and every build inside it is removed. No-op when the folder does not exist. Used by the library's folder delete action.
  const library = readLibrary()
  const folder = library.folders.find((f) => f.id === folderId)
  if (!folder) return

  if (opts.cascade) {
    const toDelete = collectDescendants(folderId, library.folders)
    library.folders = library.folders.filter((f) => !toDelete.has(f.id))
    library.builds = library.builds.filter(
      (b) => b.folderId === null || !toDelete.has(b.folderId),
    )
  } else {
    for (const f of library.folders) {
      if (f.parentId === folderId) f.parentId = folder.parentId
    }
    for (const b of library.builds) {
      if (b.folderId === folderId) b.folderId = null
    }
    library.folders = library.folders.filter((f) => f.id !== folderId)
  }

  writeLibrary(library)
}
