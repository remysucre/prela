WITH UserPostStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(p.Id) AS TotalPosts,
        SUM(CASE WHEN p.PostTypeId = 1 THEN 1 ELSE 0 END) AS Questions,
        SUM(CASE WHEN p.PostTypeId = 2 THEN 1 ELSE 0 END) AS Answers,
        SUM(p.Score) AS TotalScore,
        SUM(epoch_us(cast('2024-10-01 12:34:56' as timestamp)) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.CreationDate) / 1e6 / 3600 AS AvgPostAgeHours
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    WHERE 
        u.Reputation > 1000
    GROUP BY 
        u.Id, u.DisplayName
),
PopularTags AS (
    SELECT 
        unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS TagName
    FROM 
        Posts p
    WHERE 
        p.PostTypeId = 1
),
TagUsage AS (
    SELECT 
        t.TagName,
        COUNT(*) AS UsageCount
    FROM 
        PopularTags pt
    JOIN 
        Tags t ON t.TagName = pt.TagName
    GROUP BY 
        t.TagName
)
SELECT 
    ups.DisplayName,
    ups.TotalPosts,
    ups.Questions,
    ups.Answers,
    ups.TotalScore,
    ups.AvgPostAgeHours,
    tu.TagName,
    tu.UsageCount
FROM 
    UserPostStats ups
JOIN 
    TagUsage tu ON tu.UsageCount = (
        SELECT 
            MAX(UsageCount) 
        FROM 
            TagUsage 
    )
ORDER BY 
    ups.TotalPosts DESC, 
    ups.TotalScore DESC;