const { readFileSync } = require('fs');
const { join } = require('path');

// Mock API response structure (actual from Federation Service)
const mockApiResponse = {
  "workflow_id": "df82540b-ab97-4ff4-b5d1-672b7cf597b0",
  "status": "completed",
  "progress": 100,
  "current_step": "completed",
  "results": {
    "blog_post": {
      "content": "# Introduction\n\nIn today's rapidly evolving landscape, Write a blog post about AI automation benefits has become a critical focus for organizations worldwide. This comprehensive guide explores the key strategies, methodologies, and best practices that industry leaders are using to drive innovation and achieve sustainable success.\n\n## Key Benefits\n\n1. **Enhanced Efficiency**: Streamlined processes that reduce operational overhead\n2. **Improved Outcomes**: Data-driven approaches that deliver measurable results\n3. **Strategic Advantage**: Competitive positioning through innovative solutions\n4. **Risk Mitigation**: Proven frameworks that minimize uncertainty\n\n## Implementation Strategy\n\nSuccessful implementation requires a structured approach that considers both technical and organizational factors. Our research indicates that organizations following these principles achieve 40% better outcomes compared to traditional approaches.\n\n### Phase 1: Assessment and Planning\n\nBegin with a comprehensive evaluation of current capabilities and strategic objectives. This foundation ensures that subsequent efforts align with organizational goals and available resources.\n\n### Phase 2: Pilot Implementation\n\nStart with a focused pilot program that demonstrates value while minimizing risk. This approach allows for iterative refinement and stakeholder buy-in before full-scale deployment.\n\n### Phase 3: Scale and Optimize\n\nLeverage lessons learned from the pilot to drive organization-wide adoption. Continuous monitoring and optimization ensure sustained value delivery.\n\n## Conclusion\n\nThe journey toward Write a blog post about AI automation benefits excellence requires commitment, strategic thinking, and systematic execution. Organizations that embrace these principles position themselves for long-term success in an increasingly competitive marketplace.",
      "content_markdown": "# The Complete Guide to Write a blog post about AI automation benefits\n\nComprehensive analysis and best practices...",
      "meta_description": "Discover the essential strategies and best practices for Write a blog post about AI automation benefits implementation. Expert insights and proven methodologies for driving innovation and achieving sustainable results.",
      "seo_keywords": [
        "write a blog post about ai automation benefits",
        "best practices",
        "implementation",
        "strategy",
        "innovation"
      ],
      "title": "The Complete Guide to Write a blog post about AI automation benefits: Innovation and Best Practices",
      "word_count": 847
    },
    "image": {
      "alt_text": "Professional illustration representing Write a blog post about AI automation benefits",
      "description": "AI-generated featured image for 'The Complete Guide to Write a blog post about AI automation benefits: Innovation and Best Practices'",
      "file_size": 245760,
      "format": "JPEG",
      "height": 630,
      "url": "https://ai-generated-images.example.com/write-a-blog-post-about-ai-automation-benefits.jpg",
      "width": 1200
    },
    "metrics": {
      "execution_time_ms": 1757704892319,
      "intent_confidence": 0.95,
      "processing_steps": 5
    },
    "quality_scores": {
      "content_quality": 4.32,
      "overall_score": 4.8,
      "readability_score": 4.224,
      "seo_score": 4.56
    }
  },
  "error": null
};

// Simulate the transformation function (copy from aiCoreClient.ts)
function transformBlogPostResponse(apiResponse) {
    const blogPost = apiResponse.results?.blog_post;
    const qualityScores = apiResponse.results?.quality_scores;
    const metrics = apiResponse.results?.metrics;
    const image = apiResponse.results?.image;

    return {
        workflow_id: apiResponse.workflow_id || "",
        status: apiResponse.status || "unknown",
        progress: apiResponse.progress || 0,
        current_step: apiResponse.current_step || null,
        steps: apiResponse.steps || [],
        results: blogPost
            ? {
                  content: {
                      title: blogPost.title,
                      content: blogPost.content,
                      summary: blogPost.meta_description || "",
                      word_count: blogPost.word_count || 0,
                      reading_time: blogPost.reading_time || 0,
                      seo_keywords: blogPost.seo_keywords || [],
                      featured_image_url: image?.url,
                      meta_description: blogPost.meta_description || "",
                      tags: blogPost.seo_keywords || [],
                  },
                  images: image
                      ? [
                            {
                                url: image.url,
                                alt_text: image.alt_text,
                                width: image.width || 0,
                                height: image.height || 0,
                                format: image.format,
                                size_bytes: image.file_size || 0,
                            },
                        ]
                      : [],
                  metadata: {
                      generated_at: new Date().toISOString(),
                      model_used: "gemini-flash-1.5",
                      tokens_consumed: 0,
                      cost_usd: 0,
                      processing_time_ms: metrics?.execution_time_ms || 0,
                  },
                  quality_score: qualityScores?.overall_score || 0,
                  execution_metrics: {
                      total_duration_ms: metrics?.execution_time_ms || 0,
                      api_calls_made: metrics?.processing_steps || 3,
                      tokens_consumed: 0,
                      cost_breakdown: {
                          text_generation: 0,
                          image_generation: 0,
                          api_calls: 0,
                          total: 0,
                      },
                      performance_score: qualityScores?.overall_score || 0,
                  },
              }
            : undefined,
        error: apiResponse.error || null,
    };
}

// Test the transformation
console.log('🧪 Testing Data Transformation Fix...\n');

const transformed = transformBlogPostResponse(mockApiResponse);

console.log('✅ Transformation Results:');
console.log('Workflow ID:', transformed.workflow_id);
console.log('Status:', transformed.status);
console.log('Progress:', transformed.progress);

if (transformed.results?.content) {
    console.log('\n📝 Content Data:');
    console.log('Title:', transformed.results.content.title?.substring(0, 50) + '...');
    console.log('Content Length:', transformed.results.content.content?.length || 0, 'characters');
    console.log('Word Count:', transformed.results.content.word_count);
    console.log('Featured Image URL:', transformed.results.content.featured_image_url ? '✅ Present' : '❌ Missing');

    console.log('\n📊 Quality Score:', transformed.results.quality_score);
    console.log('Execution Time:', transformed.results.execution_metrics.total_duration_ms, 'ms');

    console.log('\n🖼️  Images:');
    console.log('Image Count:', transformed.results.images?.length || 0);
    if (transformed.results.images?.length > 0) {
        console.log('First Image URL:', transformed.results.images[0].url);
        console.log('Alt Text:', transformed.results.images[0].alt_text);
    }
} else {
    console.log('❌ No content data found in results');
}

console.log('\n🔍 Expected Client Access Paths:');
console.log('Content HTML: state.currentWorkflow.results?.content?.content');
console.log('Word Count: state.currentWorkflow.results?.content?.word_count');
console.log('Quality Score: state.currentWorkflow.results?.quality_score');

console.log('\n✨ Test Complete! The transformation should now properly map API response to client format.');
