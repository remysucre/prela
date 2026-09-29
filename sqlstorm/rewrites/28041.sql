WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Tags,
        u.DisplayName AS OwnerDisplayName,
        p.ViewCount,
        p.Score,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS PostRank
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.PostTypeId = 1 
        AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
), 
TopRankedPosts AS (
    SELECT 
        PostId,
        Title,
        CreationDate,
        Tags,
        OwnerDisplayName,
        ViewCount,
        Score
    FROM 
        RankedPosts
    WHERE 
        PostRank = 1
), 
TagUsage AS (
    SELECT 
        t.TagName,
        COUNT(DISTINCT p.Id) AS PostCount
    FROM 
        Posts p
    JOIN 
        LATERAL unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS t(TagName) ON TRUE
    WHERE 
        p.PostTypeId = 1 
    GROUP BY 
        t.TagName
), 
MostUsedTags AS (
    SELECT 
        TagName,
        PostCount,
        RANK() OVER (ORDER BY PostCount DESC) AS TagRank
    FROM 
        TagUsage
    WHERE 
        PostCount > 100
)
SELECT 
    trp.OwnerDisplayName,
    trp.Title,
    trp.CreationDate,
    trp.ViewCount,
    trp.Score,
    mut.TagName AS MostUsedTag,
    mut.PostCount AS MostUsedTagPostCount
FROM 
    TopRankedPosts trp
JOIN 
    MostUsedTags mut ON trp.Tags LIKE '%' || mut.TagName || '%'
ORDER BY 
    trp.Score DESC, trp.ViewCount DESC;