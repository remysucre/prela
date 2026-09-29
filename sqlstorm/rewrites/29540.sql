
WITH PostTagCounts AS (
    
    SELECT 
        p.Id AS PostId,
        unnest(string_to_array(substring(p.Tags, 2, length(p.Tags) - 2), '><')) AS Tag
    FROM 
        Posts p
    WHERE 
        p.PostTypeId = 1  
),
TagCounts AS (
    
    SELECT 
        Tag,
        COUNT(*) AS Count
    FROM 
        PostTagCounts
    GROUP BY 
        Tag
),
TopTags AS (
    
    SELECT 
        Tag
    FROM 
        TagCounts
    ORDER BY 
        Count DESC
    LIMIT 10
),
PostDetails AS (
    
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        ARRAY_AGG(DISTINCT tt.Tag) AS Tags
    FROM 
        Posts p
    JOIN 
        PostTagCounts ptc ON p.Id = ptc.PostId
    JOIN 
        TopTags tt ON ptc.Tag = tt.Tag
    GROUP BY 
        p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount
),
AuthorInfo AS (
    
    SELECT 
        pd.PostId,
        pd.Title,
        pd.CreationDate,
        pd.Score,
        pd.ViewCount,
        pd.Tags,
        u.DisplayName AS AuthorDisplayName,
        u.Reputation AS AuthorReputation,
        COUNT(c.Id) AS CommentCount
    FROM 
        PostDetails pd
    LEFT JOIN 
        Users u ON pd.PostId = u.Id
    LEFT JOIN 
        Comments c ON pd.PostId = c.PostId
    GROUP BY 
        pd.PostId, pd.Title, pd.CreationDate, pd.Score, pd.ViewCount, pd.Tags, u.DisplayName, u.Reputation
),
FinalResult AS (
    
    SELECT 
        AuthorDisplayName,
        AuthorReputation,
        Title,
        CreationDate,
        Score,
        ViewCount,
        Tags,
        CommentCount
    FROM 
        AuthorInfo
)
SELECT * 
FROM FinalResult
ORDER BY 
    ViewCount DESC, Score DESC, CreationDate, Title
LIMIT 20;
