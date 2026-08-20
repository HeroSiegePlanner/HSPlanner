import {
  Modal,
  MODAL_BTN_CLASS,
  MODAL_BTN_PRIMARY_CLASS,
  MODAL_FOOTER_CLASS,
} from '../ui/Modal'

export type DeepLinkPromptState =
  | { kind: 'confirm'; title: string; onConfirm: () => void }
  | { kind: 'error'; message: string }

export function DeepLinkPrompt({
  state,
  onClose,
}: {
  state: DeepLinkPromptState
  onClose: () => void
}) {
  return (
    <Modal
      onClose={onClose}
      panelClassName="w-[416px] max-w-[92vw]"
      eyebrow="Shared build"
      title={state.kind === 'confirm' ? 'Import from web?' : 'Could not open link'}
    >
      <div className="p-5">
        <p className="text-[12px] text-muted">
          {state.kind === 'confirm' ? (
            <>
              A link opened a shared build: <span className="text-text">{state.title}</span>.
              Import it into your library?
            </>
          ) : (
            state.message
          )}
        </p>
      </div>
      <div className={MODAL_FOOTER_CLASS}>
        <button type="button" onClick={onClose} className={MODAL_BTN_CLASS}>
          {state.kind === 'confirm' ? 'Cancel' : 'Close'}
        </button>
        {state.kind === 'confirm' && (
          <button
            type="button"
            onClick={() => {
              state.onConfirm()
              onClose()
            }}
            className={MODAL_BTN_PRIMARY_CLASS}
          >
            Import
          </button>
        )}
      </div>
    </Modal>
  )
}
