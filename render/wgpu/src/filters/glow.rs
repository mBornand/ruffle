use crate::backend::RenderTargetMode;
use crate::buffer_pool::TexturePool;
use crate::descriptors::Descriptors;
use crate::filters::blur::BlurFilter;
use crate::filters::{FilterSource, VERTEX_BUFFERS_DESCRIPTION_FILTERS_WITH_BLUR};
use crate::surface::target::CommandTarget;
use crate::utils::SampleCountMap;
use bytemuck::{Pod, Zeroable};
use std::sync::OnceLock;
use swf::{GlowFilter as GlowFilterArgs, Rectangle};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
struct GlowUniform {
    color: [f32; 4],
    strength: f32,
    inner: u32,            // a wasteful bool, but we need to be aligned anyway
    knockout: u32,         // a wasteful bool, but we need to be aligned anyway
    composite_source: u32, // undocumented flash feature, another bool
}

pub struct GlowFilter {
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: SampleCountMap<OnceLock<wgpu::RenderPipeline>>,
}

impl GlowFilter {
    pub fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<GlowUniform>() as u64
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
            label: create_debug_label!("Blur filter binds").as_deref(),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        Self {
            pipeline: Default::default(),
            pipeline_layout,
            bind_group_layout,
        }
    }

    fn pipeline(&self, descriptors: &Descriptors, msaa_sample_count: u32) -> &wgpu::RenderPipeline {
        self.pipeline.get_or_init(msaa_sample_count, || {
            let label = create_debug_label!("Glow Filter ({} msaa)", msaa_sample_count);
            descriptors
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: label.as_deref(),
                    layout: Some(&self.pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &descriptors.shaders.glow_filter,
                        entry_point: "main_vertex",
                        buffers: &VERTEX_BUFFERS_DESCRIPTION_FILTERS_WITH_BLUR,
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        polygon_mode: wgpu::PolygonMode::default(),
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: msaa_sample_count,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &descriptors.shaders.glow_filter,
                        entry_point: "main_fragment",
                        targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
                    }),
                    multiview: None,
                })
        })
    }

    pub fn calculate_dest_rect(
        &self,
        filter: &GlowFilterArgs,
        source_rect: Rectangle<i32>,
        blur_filter: &BlurFilter,
    ) -> Rectangle<i32> {
        // TODO: Inner might not need this. Docs suggest it doesn't care about source rect, but rather source *size*?
        blur_filter.calculate_dest_rect(&filter.inner_blur_filter(), source_rect)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        descriptors: &Descriptors,
        texture_pool: &mut TexturePool,
        draw_encoder: &mut wgpu::CommandEncoder,
        source: &FilterSource,
        filter: &GlowFilterArgs,
        blur_filter: &BlurFilter,
        blur_offset: (f32, f32),
    ) -> CommandTarget {
        let sample_count = source.texture.sample_count();
        let format = source.texture.format();
        let pipeline = self.pipeline(descriptors, sample_count);
        let blurred = blur_filter.apply(
            descriptors,
            texture_pool,
            draw_encoder,
            source,
            &filter.inner_blur_filter(),
        );
        let blurred_texture = if let Some(blurred) = &blurred {
            blurred.ensure_cleared(draw_encoder);
            blurred.color_texture()
        } else {
            source.texture
        };
        let source_view = source.texture.create_view(&Default::default());
        let blurred_view = blurred_texture.create_view(&Default::default());

        let target = CommandTarget::new(
            descriptors,
            texture_pool,
            wgpu::Extent3d {
                width: source.size.0,
                height: source.size.1,
                depth_or_array_layers: 1,
            },
            format,
            sample_count,
            RenderTargetMode::FreshWithColor(wgpu::Color::TRANSPARENT),
            draw_encoder,
        );
        let buffer = descriptors
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: create_debug_label!("Filter arguments").as_deref(),
                contents: bytemuck::cast_slice(&[GlowUniform {
                    color: [
                        f32::from(filter.color.r) / 255.0,
                        f32::from(filter.color.g) / 255.0,
                        f32::from(filter.color.b) / 255.0,
                        f32::from(filter.color.a) / 255.0,
                    ],
                    strength: filter.strength.to_f32(),
                    inner: if filter.is_inner() { 1 } else { 0 },
                    knockout: if filter.is_knockout() { 1 } else { 0 },
                    composite_source: if filter.composite_source() { 1 } else { 0 },
                }]),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let vertices = source.vertices_with_blur_offset(&descriptors.device, blur_offset);
        let filter_group = descriptors
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: create_debug_label!("Filter group").as_deref(),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&source_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            descriptors.bitmap_samplers.get_sampler(false, false),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&blurred_view),
                    },
                ],
            });
        let mut render_pass = draw_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: create_debug_label!("Glow filter").as_deref(),
            color_attachments: &[target.color_attachments()],
            depth_stencil_attachment: None,
        });
        render_pass.set_pipeline(pipeline);

        render_pass.set_bind_group(0, &filter_group, &[]);

        render_pass.set_vertex_buffer(0, vertices.slice(..));
        render_pass.set_index_buffer(
            descriptors.quad.indices.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..6, 0, 0..1);
        drop(render_pass);
        target
    }
}
