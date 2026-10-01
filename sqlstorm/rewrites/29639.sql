
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId, 
        p.Title, 
        p.Body, 
        p.Tags, 
        p.CreationDate, 
        p.Score, 
        u.DisplayName AS OwnerDisplayName,
        COUNT(a.Id) AS AnswerCount,
        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    LEFT JOIN 
        Posts a ON p.Id = a.ParentId 
    WHERE 
        p.PostTypeId = 1  
    GROUP BY 
        p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.Score, u.DisplayName
), 
PostTagCounts AS (
    SELECT 
        p.Id AS PostId, 
        UNNEST(string_to_array(substring(p.Tags, 2, LENGTH(p.Tags) - 2), '><')) AS Tag
    FROM 
        Posts p
    WHERE 
        p.PostTypeId = 1  
),
TagStats AS (
    SELECT 
        Tag, 
        COUNT(*) AS TagUsageCount
    FROM 
        PostTagCounts
    GROUP BY 
        Tag
),
TopTags AS (
    SELECT 
        Tag
    FROM 
        TagStats
    ORDER BY 
        TagUsageCount DESC
    LIMIT 10  
)
SELECT 
    rp.PostId,
    rp.Title,
    rp.Body,
    rp.CreationDate,
    rp.Score,
    rp.OwnerDisplayName,
    rp.AnswerCount,
    (SELECT STRING_AGG(tt.Tag, ', ' ORDER BY tt.Tag) FROM TopTags tt JOIN PostTagCounts pt ON tt.Tag = pt.Tag WHERE pt.PostId = rp.PostId) AS PopularTags
FROM 
    RankedPosts rp
WHERE 
    rp.rn = 1  
ORDER BY 
    rp.CreationDate DESC
LIMIT 20;
