"""Reuse the strict source loot qualifier for the exact native Giant Spider table."""
import questpc7_hot_corpse_loot as custody


def apply(root, generated, profile_path, manifest_path, canonical):
    custody.apply(root, generated, profile_path, manifest_path, canonical,
        sources={'giant_spider': (5977, 30, 13, 'fb32d55c51defaa2f87b7625ab25aa5fcb219d210755416959db6dced607a1f4')})
