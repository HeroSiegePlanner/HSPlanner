import { useState } from 'react'
import { isImageUrl } from '../utils/imageUrl'

const DEFAULT_ICON = '✦'

export function SkillIconImage({
  icon,
  size,
  className,
}: {
  icon?: string
  size?: number
  className?: string
}) {
  // Renders a skill icon. When `icon` looks like a URL/image path it is rendered as an `<img>` (lazy-loaded, draggable disabled, no referrer); on load failure or for emoji/text icons it falls back to a `<span>` with the supplied glyph (or a default ✦). Used by SkillsView, BottomBar, and tooltips wherever a skill icon needs to render uniformly.
  const [errored, setErrored] = useState(false)
  if (isImageUrl(icon) && !errored) {
    const dim = size ? `${size * 0.9}px` : '100%'
    return (
      <img
        src={icon}
        alt=""
        className={className}
        style={{
          width: dim,
          height: dim,
          objectFit: 'contain',
          display: 'block',
        }}
        draggable={false}
        loading="lazy"
        referrerPolicy="no-referrer"
        onError={() => {
          console.warn('Skill icon failed to load:', icon)
          setErrored(true)
        }}
      />
    )
  }
  return (
    <span className={className}>
      {icon && !isImageUrl(icon) ? icon : DEFAULT_ICON}
    </span>
  )
}
